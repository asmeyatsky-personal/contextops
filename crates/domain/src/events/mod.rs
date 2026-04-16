use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Base domain event type.
/// All domain events are immutable and carry their aggregate ID + timestamp.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainEventEnvelope {
    pub event_id: Uuid,
    pub aggregate_id: String,
    pub occurred_at: DateTime<Utc>,
    pub event_type: String,
    pub payload: serde_json::Value,
}

impl DomainEventEnvelope {
    pub fn new(aggregate_id: String, event_type: impl Into<String>, payload: serde_json::Value) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            aggregate_id,
            occurred_at: Utc::now(),
            event_type: event_type.into(),
            payload,
        }
    }
}

/// Domain events for the Context Registry bounded context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegistryEvent {
    ArtifactRegistered {
        artifact_id: Uuid,
        name: String,
        tier: String,
        content_hash: String,
    },
    ArtifactVersionCreated {
        artifact_id: Uuid,
        version: u64,
        content_hash: String,
    },
    ArtifactPromoted {
        artifact_id: Uuid,
        from_environment: String,
        to_environment: String,
        vaid_id: Uuid,
    },
    ArtifactDeprecated {
        artifact_id: Uuid,
        reason: String,
    },
    VaidIssued {
        vaid_id: Uuid,
        agent_id: String,
        context_hash: String,
    },
    VaidRevoked {
        vaid_id: Uuid,
        reason: String,
    },
}

/// Domain events for the Context Pipeline bounded context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PipelineEvent {
    PipelineCreated {
        pipeline_id: Uuid,
        name: String,
        stage_count: usize,
    },
    PipelineRunStarted {
        run_id: Uuid,
        pipeline_id: Uuid,
        trigger: String,
    },
    StageCompleted {
        run_id: Uuid,
        stage_name: String,
        status: String,
        duration_ms: u64,
    },
    StageFailed {
        run_id: Uuid,
        stage_name: String,
        error: String,
    },
    PipelineRunCompleted {
        run_id: Uuid,
        pipeline_id: Uuid,
        status: String,
        total_duration_ms: u64,
    },
    BlastRadiusComputed {
        artifact_id: Uuid,
        affected_agents: Vec<String>,
        affected_workflows: Vec<String>,
    },
    RollbackTriggered {
        run_id: Uuid,
        reason: String,
        previous_vaid: Uuid,
    },
}

impl RegistryEvent {
    pub fn into_envelope(self, aggregate_id: String) -> DomainEventEnvelope {
        let event_type = match &self {
            Self::ArtifactRegistered { .. } => "registry.artifact_registered",
            Self::ArtifactVersionCreated { .. } => "registry.artifact_version_created",
            Self::ArtifactPromoted { .. } => "registry.artifact_promoted",
            Self::ArtifactDeprecated { .. } => "registry.artifact_deprecated",
            Self::VaidIssued { .. } => "registry.vaid_issued",
            Self::VaidRevoked { .. } => "registry.vaid_revoked",
        };
        DomainEventEnvelope::new(
            aggregate_id,
            event_type,
            serde_json::to_value(&self).expect("event serialization"),
        )
    }
}

impl PipelineEvent {
    pub fn into_envelope(self, aggregate_id: String) -> DomainEventEnvelope {
        let event_type = match &self {
            Self::PipelineCreated { .. } => "pipeline.created",
            Self::PipelineRunStarted { .. } => "pipeline.run_started",
            Self::StageCompleted { .. } => "pipeline.stage_completed",
            Self::StageFailed { .. } => "pipeline.stage_failed",
            Self::PipelineRunCompleted { .. } => "pipeline.run_completed",
            Self::BlastRadiusComputed { .. } => "pipeline.blast_radius_computed",
            Self::RollbackTriggered { .. } => "pipeline.rollback_triggered",
        };
        DomainEventEnvelope::new(
            aggregate_id,
            event_type,
            serde_json::to_value(&self).expect("event serialization"),
        )
    }
}
