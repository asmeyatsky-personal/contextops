use std::sync::Arc;
use uuid::Uuid;

use contextops_domain::errors::DomainError;

use crate::application::dtos::{PipelineRunDetailDto, PipelineRunDto};
use crate::domain::ports::pipeline_repository::PipelineRunRepositoryPort;

/// Query to get pipeline run status and details.
pub struct GetPipelineRunQuery {
    run_repo: Arc<dyn PipelineRunRepositoryPort>,
}

impl GetPipelineRunQuery {
    pub fn new(run_repo: Arc<dyn PipelineRunRepositoryPort>) -> Self {
        Self { run_repo }
    }

    pub async fn by_id(&self, run_id: Uuid) -> Result<PipelineRunDetailDto, DomainError> {
        let run = self
            .run_repo
            .find_run_by_id(run_id)
            .await
            .map_err(|e| DomainError::InvalidContent { reason: e.to_string() })?
            .ok_or(DomainError::PipelineRunNotFound { id: run_id })?;

        Ok(PipelineRunDetailDto::from(&run))
    }

    pub async fn recent(&self, limit: usize) -> Result<Vec<PipelineRunDto>, DomainError> {
        let runs = self
            .run_repo
            .list_recent_runs(limit)
            .await
            .map_err(|e| DomainError::InvalidContent { reason: e.to_string() })?;

        Ok(runs.iter().map(PipelineRunDto::from).collect())
    }
}
