use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::DomainError;
use crate::events::RegistryEvent;
use crate::value_objects::{ContentHash, ContextTier, Vaid};

use super::ContextFormat;

/// A versioned context artifact in the Context Registry.
///
/// This is the core aggregate of the ContextRegistry bounded context.
/// Context artifacts are immutable once committed — all changes produce
/// new versions. The aggregate maintains the version history and enforces
/// invariants around tier compliance, naming, and schema validity.
///
/// Immutable domain model: state changes return new instances + domain events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextArtifact {
    id: Uuid,
    name: String,
    namespace: String,
    tier: ContextTier,
    format: ContextFormat,
    owner: String,
    current_version: u64,
    versions: Vec<ContextVersion>,
    active_vaid: Option<Vaid>,
    deprecated: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    domain_events: Vec<RegistryEvent>,
}

/// An immutable version snapshot of a context artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextVersion {
    version: u64,
    content_hash: ContentHash,
    content_size_bytes: u64,
    author: String,
    commit_sha: Option<String>,
    message: String,
    created_at: DateTime<Utc>,
}

impl ContextVersion {
    pub fn new(
        version: u64,
        content: &[u8],
        author: String,
        message: String,
        commit_sha: Option<String>,
    ) -> Self {
        Self {
            version,
            content_hash: ContentHash::from_content(content),
            content_size_bytes: content.len() as u64,
            author,
            commit_sha,
            message,
            created_at: Utc::now(),
        }
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    pub fn content_hash(&self) -> &ContentHash {
        &self.content_hash
    }

    pub fn author(&self) -> &str {
        &self.author
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn content_size_bytes(&self) -> u64 {
        self.content_size_bytes
    }
}

impl ContextArtifact {
    /// Register a new context artifact with its initial content.
    pub fn register(
        name: String,
        namespace: String,
        tier: ContextTier,
        format: ContextFormat,
        owner: String,
        content: &[u8],
        author: String,
        message: String,
    ) -> Result<Self, DomainError> {
        if name.is_empty() {
            return Err(DomainError::InvalidContent {
                reason: "artifact name cannot be empty".into(),
            });
        }
        if content.is_empty() {
            return Err(DomainError::InvalidContent {
                reason: "artifact content cannot be empty".into(),
            });
        }

        let id = Uuid::new_v4();
        let content_hash = ContentHash::from_content(content);
        let initial_version = ContextVersion::new(1, content, author, message, None);

        let event = RegistryEvent::ArtifactRegistered {
            artifact_id: id,
            name: name.clone(),
            tier: tier.to_string(),
            content_hash: content_hash.as_str().to_string(),
        };

        let now = Utc::now();
        Ok(Self {
            id,
            name,
            namespace,
            tier,
            format,
            owner,
            current_version: 1,
            versions: vec![initial_version],
            active_vaid: None,
            deprecated: false,
            created_at: now,
            updated_at: now,
            domain_events: vec![event],
        })
    }

    /// Create a new version of this artifact. Returns a new artifact instance.
    pub fn create_version(
        mut self,
        content: &[u8],
        author: String,
        message: String,
        commit_sha: Option<String>,
    ) -> Result<Self, DomainError> {
        if self.deprecated {
            return Err(DomainError::InvalidContent {
                reason: "cannot create new versions of a deprecated artifact".into(),
            });
        }
        if content.is_empty() {
            return Err(DomainError::InvalidContent {
                reason: "artifact content cannot be empty".into(),
            });
        }

        let new_version_number = self.current_version + 1;
        let content_hash = ContentHash::from_content(content);
        let version = ContextVersion::new(new_version_number, content, author, message, commit_sha);

        let event = RegistryEvent::ArtifactVersionCreated {
            artifact_id: self.id,
            version: new_version_number,
            content_hash: content_hash.as_str().to_string(),
        };

        self.current_version = new_version_number;
        self.versions.push(version);
        self.updated_at = Utc::now();
        self.domain_events.push(event);
        Ok(self)
    }

    /// Issue a VAID binding this artifact's current version to an agent.
    pub fn bind_vaid(mut self, agent_id: String) -> Self {
        let current_hash = self
            .versions
            .last()
            .expect("artifact always has at least one version")
            .content_hash()
            .clone();

        let vaid = Vaid::issue(agent_id.clone(), current_hash.clone());
        let event = RegistryEvent::VaidIssued {
            vaid_id: vaid.id(),
            agent_id,
            context_hash: current_hash.as_str().to_string(),
        };

        self.active_vaid = Some(vaid);
        self.domain_events.push(event);
        self
    }

    /// Revoke the current VAID.
    pub fn revoke_vaid(mut self, reason: String) -> Result<Self, DomainError> {
        let vaid = self.active_vaid.take().ok_or(DomainError::VaidNotFound {
            vaid_id: Uuid::nil(),
        })?;

        let event = RegistryEvent::VaidRevoked {
            vaid_id: vaid.id(),
            reason,
        };

        self.active_vaid = Some(vaid.revoke());
        self.domain_events.push(event);
        Ok(self)
    }

    /// Mark this artifact as deprecated.
    pub fn deprecate(mut self, reason: String) -> Self {
        let event = RegistryEvent::ArtifactDeprecated {
            artifact_id: self.id,
            reason,
        };
        self.deprecated = true;
        self.updated_at = Utc::now();
        self.domain_events.push(event);
        self
    }

    // --- Accessors ---

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    pub fn tier(&self) -> ContextTier {
        self.tier
    }

    pub fn format(&self) -> ContextFormat {
        self.format
    }

    pub fn owner(&self) -> &str {
        &self.owner
    }

    pub fn current_version(&self) -> u64 {
        self.current_version
    }

    pub fn versions(&self) -> &[ContextVersion] {
        &self.versions
    }

    pub fn latest_version(&self) -> &ContextVersion {
        self.versions.last().expect("always has at least one version")
    }

    pub fn active_vaid(&self) -> Option<&Vaid> {
        self.active_vaid.as_ref()
    }

    pub fn is_deprecated(&self) -> bool {
        self.deprecated
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    /// Drain and return all pending domain events.
    pub fn take_events(&mut self) -> Vec<RegistryEvent> {
        std::mem::take(&mut self.domain_events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_artifact() -> ContextArtifact {
        ContextArtifact::register(
            "billing-rules".into(),
            "org/finance".into(),
            ContextTier::Team,
            ContextFormat::ClaudeMd,
            "finance-team".into(),
            b"# Billing Rules\n\nAll invoices must be paid within 30 days.",
            "alice".into(),
            "initial billing rules context".into(),
        )
        .unwrap()
    }

    #[test]
    fn register_creates_artifact_with_version_1() {
        let artifact = make_artifact();
        assert_eq!(artifact.name(), "billing-rules");
        assert_eq!(artifact.tier(), ContextTier::Team);
        assert_eq!(artifact.current_version(), 1);
        assert_eq!(artifact.versions().len(), 1);
        assert!(!artifact.is_deprecated());
    }

    #[test]
    fn register_rejects_empty_name() {
        let result = ContextArtifact::register(
            "".into(), "ns".into(), ContextTier::Project,
            ContextFormat::ClaudeMd, "owner".into(), b"content",
            "author".into(), "msg".into(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn register_rejects_empty_content() {
        let result = ContextArtifact::register(
            "name".into(), "ns".into(), ContextTier::Project,
            ContextFormat::ClaudeMd, "owner".into(), b"",
            "author".into(), "msg".into(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn create_version_increments_version_number() {
        let artifact = make_artifact();
        let artifact = artifact
            .create_version(
                b"# Billing Rules v2\n\nUpdated payment terms.",
                "bob".into(),
                "updated payment terms".into(),
                Some("abc123".into()),
            )
            .unwrap();
        assert_eq!(artifact.current_version(), 2);
        assert_eq!(artifact.versions().len(), 2);
    }

    #[test]
    fn create_version_on_deprecated_artifact_fails() {
        let artifact = make_artifact().deprecate("obsolete".into());
        let result = artifact.create_version(
            b"new content", "author".into(), "msg".into(), None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn bind_vaid_issues_active_vaid() {
        let artifact = make_artifact().bind_vaid("agent-001".into());
        let vaid = artifact.active_vaid().unwrap();
        assert_eq!(vaid.agent_id(), "agent-001");
        assert!(vaid.is_valid());
    }

    #[test]
    fn revoke_vaid_marks_vaid_as_revoked() {
        let artifact = make_artifact().bind_vaid("agent-001".into());
        let artifact = artifact.revoke_vaid("context updated".into()).unwrap();
        assert!(artifact.active_vaid().unwrap().is_revoked());
    }

    #[test]
    fn domain_events_are_collected() {
        let mut artifact = make_artifact();
        let events = artifact.take_events();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], RegistryEvent::ArtifactRegistered { .. }));
    }

    #[test]
    fn same_content_produces_same_hash_across_versions() {
        let content = b"identical content";
        let artifact = ContextArtifact::register(
            "test".into(), "ns".into(), ContextTier::Project,
            ContextFormat::PlainText, "owner".into(), content,
            "a".into(), "m".into(),
        ).unwrap();

        let artifact = artifact.create_version(
            content, "b".into(), "same".into(), None,
        ).unwrap();

        assert_eq!(
            artifact.versions()[0].content_hash(),
            artifact.versions()[1].content_hash()
        );
    }
}
