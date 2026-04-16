use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use async_trait::async_trait;
use uuid::Uuid;

use contextops_domain::ports::repository::RepositoryError;

use crate::domain::entities::{Pipeline, PipelineRun};
use crate::domain::ports::pipeline_repository::{PipelineRepositoryPort, PipelineRunRepositoryPort};

/// In-memory pipeline repository.
#[derive(Debug, Clone)]
pub struct InMemoryPipelineRepository {
    pipelines: Arc<RwLock<HashMap<Uuid, Pipeline>>>,
}

impl InMemoryPipelineRepository {
    pub fn new() -> Self {
        Self {
            pipelines: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryPipelineRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PipelineRepositoryPort for InMemoryPipelineRepository {
    async fn save_pipeline(&self, pipeline: &Pipeline) -> Result<(), RepositoryError> {
        let mut store = self.pipelines.write().await;
        store.insert(pipeline.id(), pipeline.clone());
        Ok(())
    }

    async fn find_pipeline_by_id(&self, id: Uuid) -> Result<Option<Pipeline>, RepositoryError> {
        let store = self.pipelines.read().await;
        Ok(store.get(&id).cloned())
    }

    async fn find_pipeline_by_name(&self, name: &str) -> Result<Option<Pipeline>, RepositoryError> {
        let store = self.pipelines.read().await;
        Ok(store.values().find(|p| p.name() == name).cloned())
    }

    async fn list_pipelines(&self) -> Result<Vec<Pipeline>, RepositoryError> {
        let store = self.pipelines.read().await;
        Ok(store.values().cloned().collect())
    }
}

/// In-memory pipeline run repository.
#[derive(Debug, Clone)]
pub struct InMemoryPipelineRunRepository {
    runs: Arc<RwLock<HashMap<Uuid, PipelineRun>>>,
}

impl InMemoryPipelineRunRepository {
    pub fn new() -> Self {
        Self {
            runs: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryPipelineRunRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PipelineRunRepositoryPort for InMemoryPipelineRunRepository {
    async fn save_run(&self, run: &PipelineRun) -> Result<(), RepositoryError> {
        let mut store = self.runs.write().await;
        store.insert(run.id(), run.clone());
        Ok(())
    }

    async fn find_run_by_id(&self, id: Uuid) -> Result<Option<PipelineRun>, RepositoryError> {
        let store = self.runs.read().await;
        Ok(store.get(&id).cloned())
    }

    async fn list_runs_by_pipeline(
        &self,
        pipeline_id: Uuid,
    ) -> Result<Vec<PipelineRun>, RepositoryError> {
        let store = self.runs.read().await;
        Ok(store
            .values()
            .filter(|r| r.pipeline_id() == pipeline_id)
            .cloned()
            .collect())
    }

    async fn list_recent_runs(&self, limit: usize) -> Result<Vec<PipelineRun>, RepositoryError> {
        let store = self.runs.read().await;
        let mut runs: Vec<_> = store.values().cloned().collect();
        runs.sort_by(|a, b| b.started_at().cmp(&a.started_at()));
        runs.truncate(limit);
        Ok(runs)
    }
}
