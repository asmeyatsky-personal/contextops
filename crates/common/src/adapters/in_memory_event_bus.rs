use std::sync::Arc;
use tokio::sync::RwLock;

use async_trait::async_trait;

use contextops_domain::events::DomainEventEnvelope;
use contextops_domain::ports::event_bus::{EventBusError, EventBusPort};

/// In-memory event bus that collects domain events.
/// Used for testing and development. Production would use Pub/Sub.
#[derive(Debug, Clone)]
pub struct InMemoryEventBus {
    events: Arc<RwLock<Vec<DomainEventEnvelope>>>,
}

impl InMemoryEventBus {
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Read all collected events (for test assertions).
    pub async fn collected_events(&self) -> Vec<DomainEventEnvelope> {
        self.events.read().await.clone()
    }

    pub async fn clear(&self) {
        self.events.write().await.clear();
    }
}

impl Default for InMemoryEventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EventBusPort for InMemoryEventBus {
    async fn publish(&self, events: Vec<DomainEventEnvelope>) -> Result<(), EventBusError> {
        let mut store = self.events.write().await;
        store.extend(events);
        Ok(())
    }
}
