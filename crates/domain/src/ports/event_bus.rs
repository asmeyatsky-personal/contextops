use async_trait::async_trait;

use crate::events::DomainEventEnvelope;

/// Port for publishing domain events.
/// Cross-boundary communication happens through domain events.
#[async_trait]
pub trait EventBusPort: Send + Sync {
    async fn publish(&self, events: Vec<DomainEventEnvelope>) -> Result<(), EventBusError>;
}

#[derive(Debug, thiserror::Error)]
pub enum EventBusError {
    #[error("Failed to publish events: {0}")]
    PublishFailed(String),
}
