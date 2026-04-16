use serde::{Deserialize, Serialize};
use std::fmt;

/// The three-tier context hierarchy as defined by ContextOps™.
///
/// Tier 1 (Organisation) > Tier 2 (Team/Domain) > Tier 3 (Project/Agent)
///
/// Lower tiers can extend but NEVER override higher-tier compliance constraints.
/// This is a core invariant of the inheritance chain resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum ContextTier {
    /// Tier 1 — Organisation Context
    /// Always true for every agent. Contains: core business identity,
    /// compliance baseline, ethical constraints, fundamental operating principles.
    /// Owner: CAIO / Platform Engineering.
    /// Change process: PR with mandatory multi-approver review + full regression suite.
    Organisation,

    /// Tier 2 — Team / Domain Context
    /// True for a specific business function or domain.
    /// Contains: domain-specific workflows, terminology, policies, tool configurations.
    /// Owner: Team Lead + Context Architect.
    /// Change process: PR with domain owner approval + domain-scoped regression.
    Team,

    /// Tier 3 — Project / Agent Context
    /// True for a specific agent, workflow, or project.
    /// Contains: task-specific instructions, tool configurations, output formatting.
    /// Owner: Agent Developer.
    /// Change process: standard PR + agent-scoped regression.
    Project,
}

impl ContextTier {
    /// Returns the numeric precedence level (lower = higher authority).
    pub fn precedence(self) -> u8 {
        match self {
            Self::Organisation => 1,
            Self::Team => 2,
            Self::Project => 3,
        }
    }

    /// Returns true if this tier has higher authority than the other.
    pub fn outranks(self, other: Self) -> bool {
        self.precedence() < other.precedence()
    }
}

impl fmt::Display for ContextTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Organisation => write!(f, "tier-1/organisation"),
            Self::Team => write!(f, "tier-2/team"),
            Self::Project => write!(f, "tier-3/project"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn organisation_outranks_team_and_project() {
        assert!(ContextTier::Organisation.outranks(ContextTier::Team));
        assert!(ContextTier::Organisation.outranks(ContextTier::Project));
    }

    #[test]
    fn team_outranks_project() {
        assert!(ContextTier::Team.outranks(ContextTier::Project));
    }

    #[test]
    fn project_does_not_outrank_higher_tiers() {
        assert!(!ContextTier::Project.outranks(ContextTier::Team));
        assert!(!ContextTier::Project.outranks(ContextTier::Organisation));
    }

    #[test]
    fn same_tier_does_not_outrank_itself() {
        assert!(!ContextTier::Organisation.outranks(ContextTier::Organisation));
    }

    #[test]
    fn display_format() {
        assert_eq!(ContextTier::Organisation.to_string(), "tier-1/organisation");
        assert_eq!(ContextTier::Team.to_string(), "tier-2/team");
        assert_eq!(ContextTier::Project.to_string(), "tier-3/project");
    }
}
