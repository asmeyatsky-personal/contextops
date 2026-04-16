use async_trait::async_trait;
use uuid::Uuid;

use crate::value_objects::ContextTier;

/// Port for full-text and semantic search across the context corpus.
#[async_trait]
pub trait SearchIndexPort: Send + Sync {
    /// Index an artifact for search.
    async fn index(
        &self,
        artifact_id: Uuid,
        name: &str,
        namespace: &str,
        tier: ContextTier,
        content: &str,
    ) -> Result<(), SearchIndexError>;

    /// Full-text search across indexed artifacts.
    async fn search(
        &self,
        query: &str,
        tier_filter: Option<ContextTier>,
        limit: usize,
    ) -> Result<Vec<SearchResult>, SearchIndexError>;

    /// Remove an artifact from the search index.
    async fn remove(&self, artifact_id: Uuid) -> Result<(), SearchIndexError>;
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub artifact_id: Uuid,
    pub name: String,
    pub namespace: String,
    pub tier: ContextTier,
    pub score: f64,
    pub snippet: String,
}

#[derive(Debug, thiserror::Error)]
pub enum SearchIndexError {
    #[error("Index error: {0}")]
    IndexError(String),
}
