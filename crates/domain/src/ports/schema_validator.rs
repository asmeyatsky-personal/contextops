use async_trait::async_trait;

use crate::entities::ContextFormat;

/// Port for validating context artifact content against structural schemas.
/// Enforces structural constraints on context files before acceptance into the Registry.
#[async_trait]
pub trait SchemaValidatorPort: Send + Sync {
    /// Validate content against the expected schema for the given format.
    /// Returns a list of validation violations (empty = valid).
    async fn validate(
        &self,
        content: &[u8],
        format: ContextFormat,
    ) -> Result<Vec<SchemaViolation>, SchemaValidationError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaViolation {
    pub path: String,
    pub message: String,
    pub severity: ViolationSeverity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolationSeverity {
    Error,
    Warning,
}

#[derive(Debug, thiserror::Error)]
pub enum SchemaValidationError {
    #[error("Validation engine error: {0}")]
    EngineError(String),
}
