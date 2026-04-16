use thiserror::Error;
use uuid::Uuid;

use crate::value_objects::ContextTier;

/// Domain errors — business rule violations.
/// These are distinct from infrastructure errors (I/O, network, etc.)
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DomainError {
    #[error("Artifact not found: {id}")]
    ArtifactNotFound { id: Uuid },

    #[error("Artifact already exists with name '{name}' in tier {tier}")]
    ArtifactAlreadyExists { name: String, tier: ContextTier },

    #[error("Tier override violation: {lower_tier} cannot override constraint from {higher_tier}")]
    TierOverrideViolation {
        higher_tier: ContextTier,
        lower_tier: ContextTier,
    },

    #[error("Invalid context content: {reason}")]
    InvalidContent { reason: String },

    #[error("Schema validation failed: {violations:?}")]
    SchemaValidationFailed { violations: Vec<String> },

    #[error("VAID is revoked: {vaid_id}")]
    VaidRevoked { vaid_id: Uuid },

    #[error("VAID not found: {vaid_id}")]
    VaidNotFound { vaid_id: Uuid },

    #[error("Pipeline not found: {id}")]
    PipelineNotFound { id: Uuid },

    #[error("Pipeline run not found: {id}")]
    PipelineRunNotFound { id: Uuid },

    #[error("Pipeline stage failed: {stage_name} — {reason}")]
    PipelineStageFailed { stage_name: String, reason: String },

    #[error("Circular dependency detected in pipeline DAG")]
    CircularDependency,

    #[error("Promotion not allowed: {reason}")]
    PromotionNotAllowed { reason: String },

    #[error("Rollback failed: {reason}")]
    RollbackFailed { reason: String },

    #[error("Inheritance chain resolution failed: {reason}")]
    InheritanceResolutionFailed { reason: String },

    #[error("Concurrent modification detected on artifact {id}")]
    ConcurrentModification { id: Uuid },
}
