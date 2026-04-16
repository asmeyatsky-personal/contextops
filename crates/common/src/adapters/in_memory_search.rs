use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use async_trait::async_trait;
use uuid::Uuid;

use contextops_domain::ports::search_index::{SearchIndexError, SearchIndexPort, SearchResult};
use contextops_domain::value_objects::ContextTier;

#[derive(Debug, Clone)]
struct IndexEntry {
    artifact_id: Uuid,
    name: String,
    namespace: String,
    tier: ContextTier,
    content: String,
}

/// Simple in-memory search index using substring matching.
/// Production would use a full-text search engine.
#[derive(Debug, Clone)]
pub struct InMemorySearchIndex {
    entries: Arc<RwLock<HashMap<Uuid, IndexEntry>>>,
}

impl InMemorySearchIndex {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemorySearchIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SearchIndexPort for InMemorySearchIndex {
    async fn index(
        &self,
        artifact_id: Uuid,
        name: &str,
        namespace: &str,
        tier: ContextTier,
        content: &str,
    ) -> Result<(), SearchIndexError> {
        let mut entries = self.entries.write().await;
        entries.insert(
            artifact_id,
            IndexEntry {
                artifact_id,
                name: name.to_string(),
                namespace: namespace.to_string(),
                tier,
                content: content.to_lowercase(),
            },
        );
        Ok(())
    }

    async fn search(
        &self,
        query: &str,
        tier_filter: Option<ContextTier>,
        limit: usize,
    ) -> Result<Vec<SearchResult>, SearchIndexError> {
        let entries = self.entries.read().await;
        let query_lower = query.to_lowercase();

        let mut results: Vec<SearchResult> = entries
            .values()
            .filter(|e| {
                if let Some(tier) = tier_filter {
                    if e.tier != tier {
                        return false;
                    }
                }
                e.content.contains(&query_lower)
                    || e.name.to_lowercase().contains(&query_lower)
                    || e.namespace.to_lowercase().contains(&query_lower)
            })
            .map(|e| {
                let score = if e.name.to_lowercase().contains(&query_lower) {
                    1.0
                } else {
                    0.5
                };
                let snippet = extract_snippet(&e.content, &query_lower);
                SearchResult {
                    artifact_id: e.artifact_id,
                    name: e.name.clone(),
                    namespace: e.namespace.clone(),
                    tier: e.tier,
                    score,
                    snippet,
                }
            })
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);
        Ok(results)
    }

    async fn remove(&self, artifact_id: Uuid) -> Result<(), SearchIndexError> {
        let mut entries = self.entries.write().await;
        entries.remove(&artifact_id);
        Ok(())
    }
}

fn extract_snippet(content: &str, query: &str) -> String {
    if let Some(pos) = content.find(query) {
        let start = pos.saturating_sub(40);
        let end = (pos + query.len() + 40).min(content.len());
        format!("...{}...", &content[start..end])
    } else {
        content.chars().take(80).collect::<String>() + "..."
    }
}
