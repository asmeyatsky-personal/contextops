use std::sync::Arc;
use uuid::Uuid;

use contextops_domain::errors::DomainError;
use contextops_domain::ports::event_bus::EventBusPort;
use contextops_domain::ports::repository::ContextArtifactRepositoryPort;
use contextops_domain::ports::schema_validator::{SchemaValidatorPort, ViolationSeverity};
use contextops_domain::ports::search_index::SearchIndexPort;

use crate::application::dtos::ArtifactDto;

/// Command to create a new version of an existing context artifact.
pub struct CreateVersionCommand {
    repository: Arc<dyn ContextArtifactRepositoryPort>,
    validator: Arc<dyn SchemaValidatorPort>,
    search_index: Arc<dyn SearchIndexPort>,
    event_bus: Arc<dyn EventBusPort>,
}

pub struct CreateVersionInput {
    pub artifact_id: Uuid,
    pub content: Vec<u8>,
    pub author: String,
    pub message: String,
    pub commit_sha: Option<String>,
}

impl CreateVersionCommand {
    pub fn new(
        repository: Arc<dyn ContextArtifactRepositoryPort>,
        validator: Arc<dyn SchemaValidatorPort>,
        search_index: Arc<dyn SearchIndexPort>,
        event_bus: Arc<dyn EventBusPort>,
    ) -> Self {
        Self {
            repository,
            validator,
            search_index,
            event_bus,
        }
    }

    pub async fn execute(&self, input: CreateVersionInput) -> Result<ArtifactDto, DomainError> {
        // Load existing artifact
        let artifact = self
            .repository
            .find_by_id(input.artifact_id)
            .await
            .map_err(|e| DomainError::InvalidContent { reason: e.to_string() })?
            .ok_or(DomainError::ArtifactNotFound { id: input.artifact_id })?;

        // Validate content
        let violations = self
            .validator
            .validate(&input.content, artifact.format())
            .await
            .map_err(|e| DomainError::InvalidContent { reason: e.to_string() })?;

        let errors: Vec<String> = violations
            .iter()
            .filter(|v| v.severity == ViolationSeverity::Error)
            .map(|v| format!("{}: {}", v.path, v.message))
            .collect();

        if !errors.is_empty() {
            return Err(DomainError::SchemaValidationFailed { violations: errors });
        }

        // Create new version (immutable — returns new instance)
        let mut artifact = artifact.create_version(
            &input.content,
            input.author,
            input.message,
            input.commit_sha,
        )?;

        // Persist and re-index concurrently
        let content_hash = artifact.latest_version().content_hash().clone();
        let content_str = String::from_utf8_lossy(&input.content).to_string();

        let repo = self.repository.clone();
        let search = self.search_index.clone();
        let artifact_clone = artifact.clone();
        let artifact_id = artifact.id();
        let artifact_name = artifact.name().to_string();
        let artifact_ns = artifact.namespace().to_string();
        let artifact_tier = artifact.tier();

        let save_future = {
            let content = input.content;
            async move {
                repo.save(&artifact_clone).await?;
                repo.store_content(&content_hash, &content).await?;
                Ok::<_, contextops_domain::ports::repository::RepositoryError>(())
            }
        };

        let index_future = async move {
            search
                .index(artifact_id, &artifact_name, &artifact_ns, artifact_tier, &content_str)
                .await
                .ok();
        };

        let (save_result, _) = tokio::join!(save_future, index_future);
        save_result.map_err(|e| DomainError::InvalidContent { reason: e.to_string() })?;

        // Publish events
        let events = artifact.take_events();
        let envelopes: Vec<_> = events
            .into_iter()
            .map(|e| e.into_envelope(artifact.id().to_string()))
            .collect();
        self.event_bus.publish(envelopes).await.ok();

        Ok(ArtifactDto::from(&artifact))
    }
}
