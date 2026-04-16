use contextops_domain::entities::ContextArtifact;
use contextops_domain::errors::DomainError;
use contextops_domain::value_objects::{ContentHash, ContextTier};

/// Resolves the effective context for an agent by walking the
/// Tier 1 → Tier 2 → Tier 3 inheritance chain.
///
/// Lower tiers can extend but NEVER override higher-tier compliance constraints.
/// The resolved context is a merged view with conflict detection.
#[derive(Debug, Clone)]
pub struct ResolvedContext {
    /// The merged content from all tiers, in order of authority.
    pub layers: Vec<ContextLayer>,
    /// The composite content hash of the resolved chain.
    pub composite_hash: ContentHash,
    /// Conflicts detected during resolution.
    pub conflicts: Vec<InheritanceConflict>,
}

#[derive(Debug, Clone)]
pub struct ContextLayer {
    pub tier: ContextTier,
    pub artifact_name: String,
    pub content: Vec<u8>,
    pub content_hash: ContentHash,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InheritanceConflict {
    pub higher_tier: ContextTier,
    pub lower_tier: ContextTier,
    pub field: String,
    pub message: String,
}

/// Domain service for inheritance chain resolution.
pub struct InheritanceResolver;

impl InheritanceResolver {
    /// Resolve the inheritance chain for a given namespace path.
    ///
    /// Collects artifacts from Tier 1 (org) → Tier 2 (team) → Tier 3 (project),
    /// detects conflicts where lower tiers override higher-tier constraints,
    /// and produces a composite hash.
    pub fn resolve(
        org_artifacts: &[ContextArtifact],
        team_artifacts: &[ContextArtifact],
        project_artifacts: &[ContextArtifact],
    ) -> Result<ResolvedContext, DomainError> {
        let mut layers = Vec::new();
        let mut all_content = Vec::new();

        // Collect layers in tier order (highest authority first)
        for artifact in org_artifacts {
            let content_hash = artifact.latest_version().content_hash().clone();
            layers.push(ContextLayer {
                tier: ContextTier::Organisation,
                artifact_name: artifact.name().to_string(),
                content: Vec::new(),
                content_hash,
            });
        }

        for artifact in team_artifacts {
            let content_hash = artifact.latest_version().content_hash().clone();
            layers.push(ContextLayer {
                tier: ContextTier::Team,
                artifact_name: artifact.name().to_string(),
                content: Vec::new(),
                content_hash,
            });
        }

        for artifact in project_artifacts {
            let content_hash = artifact.latest_version().content_hash().clone();
            layers.push(ContextLayer {
                tier: ContextTier::Project,
                artifact_name: artifact.name().to_string(),
                content: Vec::new(),
                content_hash,
            });
        }

        // Build composite hash from all layer hashes
        for layer in &layers {
            all_content.extend_from_slice(layer.content_hash.as_str().as_bytes());
        }
        let composite_hash = ContentHash::from_content(&all_content);

        // Detect conflicts: lower-tier artifacts that share a name with
        // higher-tier artifacts are overrides — this violates the hierarchy.
        let conflicts = Self::detect_conflicts(org_artifacts, team_artifacts, project_artifacts);

        Ok(ResolvedContext {
            layers,
            composite_hash,
            conflicts,
        })
    }

    /// Detect naming conflicts across tiers.
    ///
    /// A lower-tier artifact with the same name as a higher-tier artifact
    /// is an override violation. Lower tiers extend, never override.
    fn detect_conflicts(
        org_artifacts: &[ContextArtifact],
        team_artifacts: &[ContextArtifact],
        project_artifacts: &[ContextArtifact],
    ) -> Vec<InheritanceConflict> {
        let mut conflicts = Vec::new();

        let org_names: std::collections::HashSet<&str> =
            org_artifacts.iter().map(|a| a.name()).collect();
        let team_names: std::collections::HashSet<&str> =
            team_artifacts.iter().map(|a| a.name()).collect();

        // Team artifacts that shadow org artifacts
        for name in &team_names {
            if org_names.contains(name) {
                conflicts.push(InheritanceConflict {
                    higher_tier: ContextTier::Organisation,
                    lower_tier: ContextTier::Team,
                    field: (*name).to_string(),
                    message: format!(
                        "Team-tier artifact '{}' shadows Organisation-tier artifact with the same name. \
                         Lower tiers must not override higher-tier context.",
                        name
                    ),
                });
            }
        }

        // Project artifacts that shadow org or team artifacts
        for artifact in project_artifacts {
            let name = artifact.name();
            if org_names.contains(name) {
                conflicts.push(InheritanceConflict {
                    higher_tier: ContextTier::Organisation,
                    lower_tier: ContextTier::Project,
                    field: name.to_string(),
                    message: format!(
                        "Project-tier artifact '{}' shadows Organisation-tier artifact with the same name. \
                         Lower tiers must not override higher-tier context.",
                        name
                    ),
                });
            } else if team_names.contains(name) {
                conflicts.push(InheritanceConflict {
                    higher_tier: ContextTier::Team,
                    lower_tier: ContextTier::Project,
                    field: name.to_string(),
                    message: format!(
                        "Project-tier artifact '{}' shadows Team-tier artifact with the same name. \
                         Lower tiers must not override higher-tier context.",
                        name
                    ),
                });
            }
        }

        conflicts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use contextops_domain::entities::{ContextArtifact, ContextFormat};

    fn make_artifact(name: &str, tier: ContextTier) -> ContextArtifact {
        ContextArtifact::register(
            name.into(),
            "test-ns".into(),
            tier,
            ContextFormat::ClaudeMd,
            "owner".into(),
            format!("# {name} content").as_bytes(),
            "author".into(),
            "initial".into(),
        )
        .unwrap()
    }

    #[test]
    fn resolve_produces_layers_in_tier_order() {
        let org = vec![make_artifact("company-rules", ContextTier::Organisation)];
        let team = vec![make_artifact("team-conventions", ContextTier::Team)];
        let project = vec![make_artifact("agent-config", ContextTier::Project)];

        let resolved = InheritanceResolver::resolve(&org, &team, &project).unwrap();

        assert_eq!(resolved.layers.len(), 3);
        assert_eq!(resolved.layers[0].tier, ContextTier::Organisation);
        assert_eq!(resolved.layers[1].tier, ContextTier::Team);
        assert_eq!(resolved.layers[2].tier, ContextTier::Project);
        assert!(resolved.conflicts.is_empty());
    }

    #[test]
    fn resolve_with_empty_tiers_succeeds() {
        let resolved = InheritanceResolver::resolve(&[], &[], &[]).unwrap();
        assert!(resolved.layers.is_empty());
        assert!(resolved.conflicts.is_empty());
    }

    #[test]
    fn composite_hash_is_deterministic() {
        let org = vec![make_artifact("rules", ContextTier::Organisation)];
        let r1 = InheritanceResolver::resolve(&org, &[], &[]).unwrap();
        let r2 = InheritanceResolver::resolve(&org, &[], &[]).unwrap();
        assert_eq!(r1.composite_hash, r2.composite_hash);
    }

    #[test]
    fn detects_team_shadowing_org() {
        let org = vec![make_artifact("security-policy", ContextTier::Organisation)];
        let team = vec![make_artifact("security-policy", ContextTier::Team)];

        let resolved = InheritanceResolver::resolve(&org, &team, &[]).unwrap();

        assert_eq!(resolved.conflicts.len(), 1);
        assert_eq!(resolved.conflicts[0].higher_tier, ContextTier::Organisation);
        assert_eq!(resolved.conflicts[0].lower_tier, ContextTier::Team);
        assert_eq!(resolved.conflicts[0].field, "security-policy");
    }

    #[test]
    fn detects_project_shadowing_org() {
        let org = vec![make_artifact("compliance-rules", ContextTier::Organisation)];
        let project = vec![make_artifact("compliance-rules", ContextTier::Project)];

        let resolved = InheritanceResolver::resolve(&org, &[], &project).unwrap();

        assert_eq!(resolved.conflicts.len(), 1);
        assert_eq!(resolved.conflicts[0].higher_tier, ContextTier::Organisation);
        assert_eq!(resolved.conflicts[0].lower_tier, ContextTier::Project);
    }

    #[test]
    fn detects_project_shadowing_team() {
        let team = vec![make_artifact("workflow", ContextTier::Team)];
        let project = vec![make_artifact("workflow", ContextTier::Project)];

        let resolved = InheritanceResolver::resolve(&[], &team, &project).unwrap();

        assert_eq!(resolved.conflicts.len(), 1);
        assert_eq!(resolved.conflicts[0].higher_tier, ContextTier::Team);
        assert_eq!(resolved.conflicts[0].lower_tier, ContextTier::Project);
    }

    #[test]
    fn no_conflict_for_different_names() {
        let org = vec![make_artifact("org-rules", ContextTier::Organisation)];
        let team = vec![make_artifact("team-conventions", ContextTier::Team)];
        let project = vec![make_artifact("agent-config", ContextTier::Project)];

        let resolved = InheritanceResolver::resolve(&org, &team, &project).unwrap();
        assert!(resolved.conflicts.is_empty());
    }

    #[test]
    fn detects_multiple_conflicts() {
        let org = vec![
            make_artifact("shared-rules", ContextTier::Organisation),
            make_artifact("compliance", ContextTier::Organisation),
        ];
        let team = vec![make_artifact("shared-rules", ContextTier::Team)];
        let project = vec![make_artifact("compliance", ContextTier::Project)];

        let resolved = InheritanceResolver::resolve(&org, &team, &project).unwrap();
        assert_eq!(resolved.conflicts.len(), 2);
    }
}
