use async_trait::async_trait;
use uuid::Uuid;

use crate::entities::ContextArtifact;
use crate::value_objects::{ContentHash, ContextTier, Vaid};

/// Port for persisting and retrieving context artifacts.
/// Defined in domain layer — implementations live in infrastructure.
#[async_trait]
pub trait ContextArtifactRepositoryPort: Send + Sync {
    /// Save a context artifact (insert or update).
    async fn save(&self, artifact: &ContextArtifact) -> Result<(), RepositoryError>;

    /// Find an artifact by its unique ID.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<ContextArtifact>, RepositoryError>;

    /// Find an artifact by name within a namespace.
    async fn find_by_name(
        &self,
        namespace: &str,
        name: &str,
    ) -> Result<Option<ContextArtifact>, RepositoryError>;

    /// List all artifacts in a given tier.
    async fn list_by_tier(&self, tier: ContextTier) -> Result<Vec<ContextArtifact>, RepositoryError>;

    /// List all artifacts in a namespace.
    async fn list_by_namespace(&self, namespace: &str) -> Result<Vec<ContextArtifact>, RepositoryError>;

    /// List all artifacts. Paginated via offset/limit.
    async fn list_all(
        &self,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<ContextArtifact>, RepositoryError>;

    /// Store raw content bytes, keyed by content hash.
    async fn store_content(
        &self,
        hash: &ContentHash,
        content: &[u8],
    ) -> Result<(), RepositoryError>;

    /// Retrieve raw content bytes by content hash.
    async fn get_content(&self, hash: &ContentHash) -> Result<Option<Vec<u8>>, RepositoryError>;

    /// Delete an artifact by ID.
    async fn delete(&self, id: Uuid) -> Result<bool, RepositoryError>;

    /// Count total artifacts.
    async fn count(&self) -> Result<usize, RepositoryError>;
}

/// Port for persisting and retrieving VAIDs.
#[async_trait]
pub trait VaidRepositoryPort: Send + Sync {
    async fn save(&self, vaid: &Vaid) -> Result<(), RepositoryError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Vaid>, RepositoryError>;
    async fn find_by_agent(&self, agent_id: &str) -> Result<Vec<Vaid>, RepositoryError>;
    async fn find_active_by_agent(&self, agent_id: &str) -> Result<Option<Vaid>, RepositoryError>;
}

#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),
}
