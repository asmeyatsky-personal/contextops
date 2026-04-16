use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use async_trait::async_trait;
use uuid::Uuid;

use contextops_domain::entities::ContextArtifact;
use contextops_domain::ports::repository::{
    ContextArtifactRepositoryPort, RepositoryError, VaidRepositoryPort,
};
use contextops_domain::value_objects::{ContentHash, ContextTier, Vaid};

/// In-memory implementation of the artifact repository port.
/// Used for development, testing, and embedded CLI mode.
#[derive(Debug, Clone)]
pub struct InMemoryArtifactRepository {
    artifacts: Arc<RwLock<HashMap<Uuid, ContextArtifact>>>,
    content_store: Arc<RwLock<HashMap<String, Vec<u8>>>>,
}

impl InMemoryArtifactRepository {
    pub fn new() -> Self {
        Self {
            artifacts: Arc::new(RwLock::new(HashMap::new())),
            content_store: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryArtifactRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ContextArtifactRepositoryPort for InMemoryArtifactRepository {
    async fn save(&self, artifact: &ContextArtifact) -> Result<(), RepositoryError> {
        let mut store = self.artifacts.write().await;
        store.insert(artifact.id(), artifact.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<ContextArtifact>, RepositoryError> {
        let store = self.artifacts.read().await;
        Ok(store.get(&id).cloned())
    }

    async fn find_by_name(
        &self,
        namespace: &str,
        name: &str,
    ) -> Result<Option<ContextArtifact>, RepositoryError> {
        let store = self.artifacts.read().await;
        Ok(store
            .values()
            .find(|a| a.namespace() == namespace && a.name() == name)
            .cloned())
    }

    async fn list_by_tier(
        &self,
        tier: ContextTier,
    ) -> Result<Vec<ContextArtifact>, RepositoryError> {
        let store = self.artifacts.read().await;
        Ok(store.values().filter(|a| a.tier() == tier).cloned().collect())
    }

    async fn list_by_namespace(
        &self,
        namespace: &str,
    ) -> Result<Vec<ContextArtifact>, RepositoryError> {
        let store = self.artifacts.read().await;
        Ok(store
            .values()
            .filter(|a| a.namespace() == namespace)
            .cloned()
            .collect())
    }

    async fn list_all(
        &self,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<ContextArtifact>, RepositoryError> {
        let store = self.artifacts.read().await;
        let mut artifacts: Vec<_> = store.values().cloned().collect();
        artifacts.sort_by_key(|a| a.updated_at());
        Ok(artifacts.into_iter().skip(offset).take(limit).collect())
    }

    async fn store_content(
        &self,
        hash: &ContentHash,
        content: &[u8],
    ) -> Result<(), RepositoryError> {
        let mut store = self.content_store.write().await;
        store.insert(hash.as_str().to_string(), content.to_vec());
        Ok(())
    }

    async fn get_content(&self, hash: &ContentHash) -> Result<Option<Vec<u8>>, RepositoryError> {
        let store = self.content_store.read().await;
        Ok(store.get(hash.as_str()).cloned())
    }

    async fn delete(&self, id: Uuid) -> Result<bool, RepositoryError> {
        let mut store = self.artifacts.write().await;
        Ok(store.remove(&id).is_some())
    }

    async fn count(&self) -> Result<usize, RepositoryError> {
        let store = self.artifacts.read().await;
        Ok(store.len())
    }
}

/// In-memory implementation of the VAID repository port.
#[derive(Debug, Clone)]
pub struct InMemoryVaidRepository {
    vaids: Arc<RwLock<HashMap<Uuid, Vaid>>>,
}

impl InMemoryVaidRepository {
    pub fn new() -> Self {
        Self {
            vaids: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryVaidRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl VaidRepositoryPort for InMemoryVaidRepository {
    async fn save(&self, vaid: &Vaid) -> Result<(), RepositoryError> {
        let mut store = self.vaids.write().await;
        store.insert(vaid.id(), vaid.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Vaid>, RepositoryError> {
        let store = self.vaids.read().await;
        Ok(store.get(&id).cloned())
    }

    async fn find_by_agent(&self, agent_id: &str) -> Result<Vec<Vaid>, RepositoryError> {
        let store = self.vaids.read().await;
        Ok(store
            .values()
            .filter(|v| v.agent_id() == agent_id)
            .cloned()
            .collect())
    }

    async fn find_active_by_agent(
        &self,
        agent_id: &str,
    ) -> Result<Option<Vaid>, RepositoryError> {
        let store = self.vaids.read().await;
        Ok(store
            .values()
            .find(|v| v.agent_id() == agent_id && v.is_valid())
            .cloned())
    }
}
