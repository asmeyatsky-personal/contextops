use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

use super::ContentHash;

/// Verifiable Agent Identity Document (VAID).
///
/// The clean primitive that binds an agent identity to its versioned
/// context snapshot. SYNTHERA integration point.
///
/// A VAID is issued when a context snapshot is promoted to an environment.
/// Agents present their VAID at runtime to prove they are operating
/// on authorised, versioned context.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Vaid {
    /// Unique identifier for this VAID.
    id: Uuid,
    /// The agent identity this VAID is bound to.
    agent_id: String,
    /// The content hash of the resolved context snapshot.
    context_hash: ContentHash,
    /// ISO 8601 timestamp of issuance.
    issued_at: chrono::DateTime<chrono::Utc>,
    /// Whether this VAID has been revoked.
    revoked: bool,
}

impl Vaid {
    pub fn issue(agent_id: String, context_hash: ContentHash) -> Self {
        Self {
            id: Uuid::new_v4(),
            agent_id,
            context_hash,
            issued_at: chrono::Utc::now(),
            revoked: false,
        }
    }

    /// Revoke this VAID. Returns a new instance with revoked=true.
    /// Immutable domain model — state changes produce new instances.
    pub fn revoke(self) -> Self {
        Self {
            revoked: true,
            ..self
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }

    pub fn context_hash(&self) -> &ContentHash {
        &self.context_hash
    }

    pub fn issued_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.issued_at
    }

    pub fn is_revoked(&self) -> bool {
        self.revoked
    }

    pub fn is_valid(&self) -> bool {
        !self.revoked
    }
}

impl fmt::Display for Vaid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status = if self.revoked { "REVOKED" } else { "ACTIVE" };
        write!(
            f,
            "VAID[{}] agent={} context={} status={}",
            self.id, self.agent_id, self.context_hash, status
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_creates_active_vaid() {
        let hash = ContentHash::from_content(b"test context");
        let vaid = Vaid::issue("agent-001".into(), hash.clone());

        assert_eq!(vaid.agent_id(), "agent-001");
        assert_eq!(vaid.context_hash(), &hash);
        assert!(vaid.is_valid());
        assert!(!vaid.is_revoked());
    }

    #[test]
    fn revoke_produces_new_instance() {
        let hash = ContentHash::from_content(b"test context");
        let vaid = Vaid::issue("agent-001".into(), hash);
        let revoked = vaid.revoke();

        assert!(revoked.is_revoked());
        assert!(!revoked.is_valid());
    }
}
