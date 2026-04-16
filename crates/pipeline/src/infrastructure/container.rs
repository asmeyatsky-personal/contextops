use std::sync::Arc;

use contextops_common::adapters::{InMemoryEventBus, PassthroughSchemaValidator};
use contextops_domain::ports::event_bus::EventBusPort;
use contextops_domain::ports::repository::ContextArtifactRepositoryPort;

use crate::application::commands::RunPipelineCommand;
use crate::application::queries::GetPipelineRunQuery;
use crate::domain::ports::stage_executor::StageExecutorPort;
use crate::domain::services::DagOrchestrator;
use crate::infrastructure::executors::{
    BlastRadiusStageExecutor, PromoteProductionExecutor, PromoteStagingExecutor,
    RegressionTestStageExecutor, SecurityScanStageExecutor, ValidateStageExecutor,
};
use crate::infrastructure::repositories::{
    InMemoryPipelineRepository, InMemoryPipelineRunRepository,
};

/// Dependency injection container for the Context Pipeline bounded context.
pub struct PipelineContainer {
    pub run_pipeline: RunPipelineCommand,
    pub get_pipeline_run: GetPipelineRunQuery,
}

impl PipelineContainer {
    /// Create a container with in-memory adapters and default stage executors.
    pub fn in_memory(artifact_repo: Arc<dyn ContextArtifactRepositoryPort>) -> Self {
        let pipeline_repo = Arc::new(InMemoryPipelineRepository::new());
        let run_repo = Arc::new(InMemoryPipelineRunRepository::new());
        let event_bus: Arc<dyn EventBusPort> = Arc::new(InMemoryEventBus::new());

        let validator = Arc::new(PassthroughSchemaValidator::new());

        // Build stage executors — wired to real ports
        let executors: Vec<Arc<dyn StageExecutorPort>> = vec![
            Arc::new(ValidateStageExecutor::new(validator)),
            Arc::new(BlastRadiusStageExecutor::new(artifact_repo.clone())),
            Arc::new(RegressionTestStageExecutor::new(artifact_repo.clone())),
            Arc::new(SecurityScanStageExecutor),
            Arc::new(PromoteStagingExecutor),
            Arc::new(PromoteProductionExecutor),
        ];

        let orchestrator = Arc::new(DagOrchestrator::new(executors));

        let run_pipeline = RunPipelineCommand::new(
            pipeline_repo,
            run_repo.clone(),
            artifact_repo,
            orchestrator,
            event_bus,
        );

        let get_pipeline_run = GetPipelineRunQuery::new(run_repo);

        Self {
            run_pipeline,
            get_pipeline_run,
        }
    }
}
