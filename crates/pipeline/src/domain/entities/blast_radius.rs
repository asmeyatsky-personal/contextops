use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Blast radius analysis result for a context change.
///
/// Shows all agents and workflows that will be affected by a
/// proposed context change, computed before deployment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlastRadius {
    /// The context artifact being changed.
    pub artifact_id: Uuid,
    /// Agents that consume this context artifact (directly or via inheritance).
    pub affected_agents: Vec<AffectedAgent>,
    /// Workflows that include this artifact in their context chain.
    pub affected_workflows: Vec<AffectedWorkflow>,
    /// Severity assessment based on tier and scope.
    pub severity: BlastSeverity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AffectedAgent {
    pub agent_id: String,
    pub agent_name: String,
    pub relationship: String, // "direct", "inherited", "transitive"
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AffectedWorkflow {
    pub workflow_id: String,
    pub workflow_name: String,
    pub stage: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlastSeverity {
    /// Tier 3 change, few affected agents
    Low,
    /// Tier 2 change, or moderate number of affected agents
    Medium,
    /// Tier 1 change, or large number of affected agents
    High,
    /// Tier 1 compliance-level change affecting all agents
    Critical,
}

impl BlastRadius {
    pub fn compute(
        artifact_id: Uuid,
        affected_agents: Vec<AffectedAgent>,
        affected_workflows: Vec<AffectedWorkflow>,
        is_tier1: bool,
    ) -> Self {
        let severity = match (is_tier1, affected_agents.len()) {
            (true, _) if affected_agents.len() > 10 => BlastSeverity::Critical,
            (true, _) => BlastSeverity::High,
            (false, n) if n > 20 => BlastSeverity::High,
            (false, n) if n > 5 => BlastSeverity::Medium,
            _ => BlastSeverity::Low,
        };

        Self {
            artifact_id,
            affected_agents,
            affected_workflows,
            severity,
        }
    }

    pub fn total_affected(&self) -> usize {
        self.affected_agents.len() + self.affected_workflows.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier1_change_is_high_severity() {
        let br = BlastRadius::compute(
            Uuid::new_v4(),
            vec![AffectedAgent {
                agent_id: "a1".into(),
                agent_name: "Agent 1".into(),
                relationship: "direct".into(),
            }],
            vec![],
            true,
        );
        assert_eq!(br.severity, BlastSeverity::High);
    }

    #[test]
    fn small_tier3_change_is_low_severity() {
        let br = BlastRadius::compute(Uuid::new_v4(), vec![], vec![], false);
        assert_eq!(br.severity, BlastSeverity::Low);
    }
}
