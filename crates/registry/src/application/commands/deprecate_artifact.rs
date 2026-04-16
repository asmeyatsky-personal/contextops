use std::sync::Arc;
use uuid::Uuid;

use contextops_domain::errors::DomainError;
use contextops_domain::ports::event_bus::EventBusPort;
use contextops_domain::ports::repository::ContextArtifactRepositoryPort;

use crate::application::dtos::ArtifactDto;

/// Command to deprecate a context artifact.
pub struct DeprecateArtifactCommand {
    repository: Arc<dyn ContextArtifactRepositoryPort>,
    event_bus: Arc<dyn EventBusPort>,
}

pub struct DeprecateArtifactInput {
    pub artifact_id: Uuid,
    pub reason: String,
}

impl DeprecateArtifactCommand {
    pub fn new(
        repository: Arc<dyn ContextArtifactRepositoryPort>,
        event_bus: Arc<dyn EventBusPort>,
    ) -> Self {
        Self {
            repository,
            event_bus,
        }
    }

    pub async fn execute(&self, input: DeprecateArtifactInput) -> Result<ArtifactDto, DomainError> {
        let artifact = self
            .repository
            .find_by_id(input.artifact_id)
            .await
            .map_err(|e| DomainError::InvalidContent { reason: e.to_string() })?
            .ok_or(DomainError::ArtifactNotFound { id: input.artifact_id })?;

        let mut artifact = artifact.deprecate(input.reason);

        self.repository
            .save(&artifact)
            .await
            .map_err(|e| DomainError::InvalidContent { reason: e.to_string() })?;

        let events = artifact.take_events();
        let envelopes: Vec<_> = events
            .into_iter()
            .map(|e| e.into_envelope(artifact.id().to_string()))
            .collect();
        self.event_bus.publish(envelopes).await.ok();

        Ok(ArtifactDto::from(&artifact))
    }
}
