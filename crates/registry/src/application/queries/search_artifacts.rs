use std::sync::Arc;

use contextops_domain::errors::DomainError;
use contextops_domain::ports::search_index::{SearchIndexPort, SearchResult};
use contextops_domain::value_objects::ContextTier;

/// Query to search across the context corpus.
pub struct SearchArtifactsQuery {
    search_index: Arc<dyn SearchIndexPort>,
}

impl SearchArtifactsQuery {
    pub fn new(search_index: Arc<dyn SearchIndexPort>) -> Self {
        Self { search_index }
    }

    pub async fn search(
        &self,
        query: &str,
        tier_filter: Option<ContextTier>,
        limit: usize,
    ) -> Result<Vec<SearchResult>, DomainError> {
        self.search_index
            .search(query, tier_filter, limit)
            .await
            .map_err(|e| DomainError::InvalidContent {
                reason: e.to_string(),
            })
    }
}
