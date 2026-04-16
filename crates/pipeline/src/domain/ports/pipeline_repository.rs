use async_trait::async_trait;
use uuid::Uuid;

use contextops_domain::ports::repository::RepositoryError;

use crate::domain::entities::{Pipeline, PipelineRun};

/// Port for persisting pipeline definitions.
#[async_trait]
pub trait PipelineRepositoryPort: Send + Sync {
    async fn save_pipeline(&self, pipeline: &Pipeline) -> Result<(), RepositoryError>;
    async fn find_pipeline_by_id(&self, id: Uuid) -> Result<Option<Pipeline>, RepositoryError>;
    async fn find_pipeline_by_name(&self, name: &str) -> Result<Option<Pipeline>, RepositoryError>;
    async fn list_pipelines(&self) -> Result<Vec<Pipeline>, RepositoryError>;
}

/// Port for persisting pipeline run state.
#[async_trait]
pub trait PipelineRunRepositoryPort: Send + Sync {
    async fn save_run(&self, run: &PipelineRun) -> Result<(), RepositoryError>;
    async fn find_run_by_id(&self, id: Uuid) -> Result<Option<PipelineRun>, RepositoryError>;
    async fn list_runs_by_pipeline(&self, pipeline_id: Uuid) -> Result<Vec<PipelineRun>, RepositoryError>;
    async fn list_recent_runs(&self, limit: usize) -> Result<Vec<PipelineRun>, RepositoryError>;
}
