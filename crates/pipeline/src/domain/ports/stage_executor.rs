use async_trait::async_trait;

use crate::domain::entities::StageKind;

/// Port for executing a pipeline stage.
/// Each stage kind has its own implementation — swappable via DI.
#[async_trait]
pub trait StageExecutorPort: Send + Sync {
    /// The kind of stage this executor handles.
    fn stage_kind(&self) -> StageKind;

    /// Execute the stage. Returns structured output on success.
    async fn execute(
        &self,
        context: &StageContext,
    ) -> Result<serde_json::Value, StageExecutorError>;
}

/// Context passed to each stage executor.
#[derive(Debug, Clone)]
pub struct StageContext {
    pub artifact_id: uuid::Uuid,
    pub content: Vec<u8>,
    pub content_hash: String,
    pub namespace: String,
    pub tier: String,
    pub previous_results: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, thiserror::Error)]
pub enum StageExecutorError {
    #[error("Stage execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Stage timed out after {0}s")]
    Timeout(u64),

    #[error("Quality gate failed: {violations:?}")]
    QualityGateFailed { violations: Vec<String> },
}
