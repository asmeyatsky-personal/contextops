use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use contextops_domain::errors::DomainError;

/// The kind of pipeline stage — maps to the PRD's defined pipeline stages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StageKind {
    /// Schema validation, lint, inheritance chain resolution, conflict detection
    Validate,
    /// Compute all agents and workflows affected by this context change
    BlastRadius,
    /// Run golden-dataset behavioural tests against affected agents
    RegressionTest,
    /// DevSecOps scan: PII exposure, secrets detection, prompt injection analysis
    SecurityScan,
    /// Deploy context to staging environment
    PromoteStaging,
    /// Blue/green deploy to production
    PromoteProduction,
    /// Automatic rollback on threshold breach
    Rollback,
}

impl std::fmt::Display for StageKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Validate => write!(f, "validate"),
            Self::BlastRadius => write!(f, "blast-radius"),
            Self::RegressionTest => write!(f, "regression-test"),
            Self::SecurityScan => write!(f, "security-scan"),
            Self::PromoteStaging => write!(f, "promote-staging"),
            Self::PromoteProduction => write!(f, "promote-production"),
            Self::Rollback => write!(f, "rollback"),
        }
    }
}

/// A single stage in a context pipeline.
/// Stages have explicit dependencies — forming a DAG.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PipelineStage {
    pub name: String,
    pub kind: StageKind,
    pub depends_on: Vec<String>,
    pub timeout_seconds: u64,
    pub required: bool,
}

/// A context pipeline definition — a DAG of stages.
///
/// Immutable domain model: once created, the pipeline definition is fixed.
/// Pipeline runs track mutable execution state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pipeline {
    id: Uuid,
    name: String,
    description: String,
    stages: Vec<PipelineStage>,
    created_at: DateTime<Utc>,
    created_by: String,
}

impl Pipeline {
    pub fn new(
        name: String,
        description: String,
        stages: Vec<PipelineStage>,
        created_by: String,
    ) -> Result<Self, DomainError> {
        if stages.is_empty() {
            return Err(DomainError::InvalidContent {
                reason: "pipeline must have at least one stage".into(),
            });
        }

        let pipeline = Self {
            id: Uuid::new_v4(),
            name,
            description,
            stages,
            created_at: Utc::now(),
            created_by,
        };

        // Validate DAG — check for cycles and missing dependencies
        pipeline.validate_dag()?;

        Ok(pipeline)
    }

    /// Create the standard ContextOps pipeline with all PRD-defined stages.
    pub fn standard(created_by: String) -> Result<Self, DomainError> {
        let stages = vec![
            PipelineStage {
                name: "validate".into(),
                kind: StageKind::Validate,
                depends_on: vec![],
                timeout_seconds: 60,
                required: true,
            },
            PipelineStage {
                name: "blast-radius".into(),
                kind: StageKind::BlastRadius,
                depends_on: vec!["validate".into()],
                timeout_seconds: 120,
                required: true,
            },
            PipelineStage {
                name: "regression-test".into(),
                kind: StageKind::RegressionTest,
                depends_on: vec!["validate".into()],
                timeout_seconds: 300,
                required: true,
            },
            PipelineStage {
                name: "security-scan".into(),
                kind: StageKind::SecurityScan,
                depends_on: vec!["validate".into()],
                timeout_seconds: 120,
                required: true,
            },
            PipelineStage {
                name: "promote-staging".into(),
                kind: StageKind::PromoteStaging,
                depends_on: vec![
                    "blast-radius".into(),
                    "regression-test".into(),
                    "security-scan".into(),
                ],
                timeout_seconds: 180,
                required: true,
            },
            PipelineStage {
                name: "promote-production".into(),
                kind: StageKind::PromoteProduction,
                depends_on: vec!["promote-staging".into()],
                timeout_seconds: 300,
                required: true,
            },
        ];

        Self::new(
            "contextops-standard".into(),
            "Standard ContextOps context deployment pipeline".into(),
            stages,
            created_by,
        )
    }

    fn validate_dag(&self) -> Result<(), DomainError> {
        let stage_names: std::collections::HashSet<&str> =
            self.stages.iter().map(|s| s.name.as_str()).collect();

        // Check all dependencies reference existing stages
        for stage in &self.stages {
            for dep in &stage.depends_on {
                if !stage_names.contains(dep.as_str()) {
                    return Err(DomainError::InvalidContent {
                        reason: format!(
                            "stage '{}' depends on '{}' which does not exist",
                            stage.name, dep
                        ),
                    });
                }
            }
        }

        // Topological sort to detect cycles
        self.topological_sort()?;

        Ok(())
    }

    /// Topological sort of stages — returns execution levels.
    /// Each level contains stages that can run concurrently.
    pub fn topological_sort(&self) -> Result<Vec<Vec<&PipelineStage>>, DomainError> {
        let mut in_degree: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        let mut dependents: std::collections::HashMap<&str, Vec<&str>> = std::collections::HashMap::new();

        for stage in &self.stages {
            in_degree.entry(stage.name.as_str()).or_insert(0);
            for dep in &stage.depends_on {
                *in_degree.entry(stage.name.as_str()).or_insert(0) += 1;
                dependents
                    .entry(dep.as_str())
                    .or_default()
                    .push(stage.name.as_str());
            }
        }

        let stage_map: std::collections::HashMap<&str, &PipelineStage> =
            self.stages.iter().map(|s| (s.name.as_str(), s)).collect();

        let mut levels: Vec<Vec<&PipelineStage>> = Vec::new();
        let mut processed = 0;

        loop {
            let ready: Vec<&str> = in_degree
                .iter()
                .filter(|(_, deg)| **deg == 0)
                .map(|(name, _)| *name)
                .collect();

            if ready.is_empty() {
                break;
            }

            let level: Vec<&PipelineStage> = ready
                .iter()
                .filter_map(|name| stage_map.get(name).copied())
                .collect();

            for name in &ready {
                in_degree.remove(name);
                if let Some(deps) = dependents.get(name) {
                    for dep in deps {
                        if let Some(degree) = in_degree.get_mut(dep) {
                            *degree -= 1;
                        }
                    }
                }
            }

            processed += level.len();
            levels.push(level);
        }

        if processed != self.stages.len() {
            return Err(DomainError::CircularDependency);
        }

        Ok(levels)
    }

    // --- Accessors ---

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn stages(&self) -> &[PipelineStage] {
        &self.stages
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn created_by(&self) -> &str {
        &self.created_by
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_pipeline_creates_valid_dag() {
        let pipeline = Pipeline::standard("test".into()).unwrap();
        assert_eq!(pipeline.name(), "contextops-standard");
        assert_eq!(pipeline.stages().len(), 6);
    }

    #[test]
    fn topological_sort_groups_parallel_stages() {
        let pipeline = Pipeline::standard("test".into()).unwrap();
        let levels = pipeline.topological_sort().unwrap();

        // Level 0: validate (no deps)
        assert_eq!(levels[0].len(), 1);
        assert_eq!(levels[0][0].name, "validate");

        // Level 1: blast-radius, regression-test, security-scan (all depend on validate)
        assert_eq!(levels[1].len(), 3);
        let level1_names: std::collections::HashSet<&str> =
            levels[1].iter().map(|s| s.name.as_str()).collect();
        assert!(level1_names.contains("blast-radius"));
        assert!(level1_names.contains("regression-test"));
        assert!(level1_names.contains("security-scan"));

        // Level 2: promote-staging (depends on all level 1)
        assert_eq!(levels[2].len(), 1);
        assert_eq!(levels[2][0].name, "promote-staging");

        // Level 3: promote-production
        assert_eq!(levels[3].len(), 1);
        assert_eq!(levels[3][0].name, "promote-production");
    }

    #[test]
    fn circular_dependency_is_detected() {
        let stages = vec![
            PipelineStage {
                name: "a".into(),
                kind: StageKind::Validate,
                depends_on: vec!["b".into()],
                timeout_seconds: 60,
                required: true,
            },
            PipelineStage {
                name: "b".into(),
                kind: StageKind::BlastRadius,
                depends_on: vec!["a".into()],
                timeout_seconds: 60,
                required: true,
            },
        ];
        let result = Pipeline::new("test".into(), "".into(), stages, "user".into());
        assert!(matches!(result, Err(DomainError::CircularDependency)));
    }

    #[test]
    fn missing_dependency_is_rejected() {
        let stages = vec![PipelineStage {
            name: "a".into(),
            kind: StageKind::Validate,
            depends_on: vec!["nonexistent".into()],
            timeout_seconds: 60,
            required: true,
        }];
        let result = Pipeline::new("test".into(), "".into(), stages, "user".into());
        assert!(result.is_err());
    }

    #[test]
    fn empty_pipeline_is_rejected() {
        let result = Pipeline::new("test".into(), "".into(), vec![], "user".into());
        assert!(result.is_err());
    }
}
