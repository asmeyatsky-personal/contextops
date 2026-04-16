use std::sync::Arc;
use uuid::Uuid;

use contextops_domain::errors::DomainError;
use contextops_domain::ports::event_bus::EventBusPort;
use contextops_domain::ports::repository::ContextArtifactRepositoryPort;

use crate::application::dtos::PipelineRunDetailDto;
use crate::domain::entities::Pipeline;
use crate::domain::ports::pipeline_repository::{PipelineRepositoryPort, PipelineRunRepositoryPort};
use crate::domain::ports::stage_executor::StageContext;
use crate::domain::services::DagOrchestrator;

pub struct RunPipelineInput {
    pub pipeline_id: Option<Uuid>,
    pub artifact_id: Uuid,
    pub trigger: String,
}

/// Command to run a context pipeline against an artifact.
pub struct RunPipelineCommand {
    pipeline_repo: Arc<dyn PipelineRepositoryPort>,
    run_repo: Arc<dyn PipelineRunRepositoryPort>,
    artifact_repo: Arc<dyn ContextArtifactRepositoryPort>,
    orchestrator: Arc<DagOrchestrator>,
    event_bus: Arc<dyn EventBusPort>,
}

impl RunPipelineCommand {
    pub fn new(
        pipeline_repo: Arc<dyn PipelineRepositoryPort>,
        run_repo: Arc<dyn PipelineRunRepositoryPort>,
        artifact_repo: Arc<dyn ContextArtifactRepositoryPort>,
        orchestrator: Arc<DagOrchestrator>,
        event_bus: Arc<dyn EventBusPort>,
    ) -> Self {
        Self {
            pipeline_repo,
            run_repo,
            artifact_repo,
            orchestrator,
            event_bus,
        }
    }

    pub async fn execute(&self, input: RunPipelineInput) -> Result<PipelineRunDetailDto, DomainError> {
        // Load pipeline definition (or use standard)
        let pipeline = match input.pipeline_id {
            Some(id) => self
                .pipeline_repo
                .find_pipeline_by_id(id)
                .await
                .map_err(|e| DomainError::InvalidContent { reason: e.to_string() })?
                .ok_or(DomainError::PipelineNotFound { id })?,
            None => {
                // Use the standard pipeline, creating it if needed
                match self.pipeline_repo.find_pipeline_by_name("contextops-standard").await {
                    Ok(Some(p)) => p,
                    _ => {
                        let p = Pipeline::standard("system".into())?;
                        self.pipeline_repo
                            .save_pipeline(&p)
                            .await
                            .map_err(|e| DomainError::InvalidContent { reason: e.to_string() })?;
                        p
                    }
                }
            }
        };

        // Load the artifact to get content for stage context
        let artifact = self
            .artifact_repo
            .find_by_id(input.artifact_id)
            .await
            .map_err(|e| DomainError::InvalidContent { reason: e.to_string() })?
            .ok_or(DomainError::ArtifactNotFound { id: input.artifact_id })?;

        let content = self
            .artifact_repo
            .get_content(artifact.latest_version().content_hash())
            .await
            .map_err(|e| DomainError::InvalidContent { reason: e.to_string() })?
            .unwrap_or_default();

        let stage_context = StageContext {
            artifact_id: artifact.id(),
            content,
            content_hash: artifact.latest_version().content_hash().as_str().to_string(),
            namespace: artifact.namespace().to_string(),
            tier: artifact.tier().to_string(),
            previous_results: std::collections::HashMap::new(),
        };

        // Execute the pipeline DAG
        let run = self.orchestrator.execute(&pipeline, stage_context).await?;

        // Persist the run
        self.run_repo
            .save_run(&run)
            .await
            .map_err(|e| DomainError::InvalidContent { reason: e.to_string() })?;

        // Publish events (take_events consumes and returns new instance)
        let run_id_str = run.id().to_string();
        let (run, events) = run.take_events();
        let envelopes: Vec<_> = events
            .into_iter()
            .map(|e| e.into_envelope(run_id_str.clone()))
            .collect();
        self.event_bus.publish(envelopes).await.ok();

        Ok(PipelineRunDetailDto::from(&run))
    }
}
