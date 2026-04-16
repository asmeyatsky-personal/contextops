use std::sync::Arc;

use contextops_domain::entities::{ContextArtifact, ContextFormat};
use contextops_domain::errors::DomainError;
use contextops_domain::ports::event_bus::EventBusPort;
use contextops_domain::ports::repository::ContextArtifactRepositoryPort;
use contextops_domain::ports::schema_validator::{SchemaValidatorPort, ViolationSeverity};
use contextops_domain::ports::search_index::SearchIndexPort;
use contextops_domain::value_objects::ContextTier;

use crate::application::dtos::ArtifactDto;

/// Command to register a new context artifact.
///
/// One use case per class — orchestrates domain objects via ports.
/// Schema validation and search indexing run concurrently (parallelism-first).
pub struct RegisterArtifactCommand {
    repository: Arc<dyn ContextArtifactRepositoryPort>,
    validator: Arc<dyn SchemaValidatorPort>,
    search_index: Arc<dyn SearchIndexPort>,
    event_bus: Arc<dyn EventBusPort>,
}

pub struct RegisterArtifactInput {
    pub name: String,
    pub namespace: String,
    pub tier: ContextTier,
    pub format: ContextFormat,
    pub owner: String,
    pub content: Vec<u8>,
    pub author: String,
    pub message: String,
}

impl RegisterArtifactCommand {
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

    pub async fn execute(&self, input: RegisterArtifactInput) -> Result<ArtifactDto, DomainError> {
        // Check for duplicate name in namespace
        let existing = self
            .repository
            .find_by_name(&input.namespace, &input.name)
            .await
            .map_err(|e| DomainError::InvalidContent {
                reason: e.to_string(),
            })?;

        if existing.is_some() {
            return Err(DomainError::ArtifactAlreadyExists {
                name: input.name,
                tier: input.tier,
            });
        }

        // Validate schema
        let violations = self
            .validator
            .validate(&input.content, input.format)
            .await
            .map_err(|e| DomainError::InvalidContent {
                reason: e.to_string(),
            })?;

        let errors: Vec<String> = violations
            .iter()
            .filter(|v| v.severity == ViolationSeverity::Error)
            .map(|v| format!("{}: {}", v.path, v.message))
            .collect();

        if !errors.is_empty() {
            return Err(DomainError::SchemaValidationFailed { violations: errors });
        }

        // Create the domain entity
        let mut artifact = ContextArtifact::register(
            input.name,
            input.namespace,
            input.tier,
            input.format,
            input.owner,
            &input.content,
            input.author,
            input.message,
        )?;

        // Persist artifact and index — run concurrently (parallelism-first)
        let content_hash = artifact.latest_version().content_hash().clone();
        let content_str = String::from_utf8_lossy(&input.content).to_string();

        let repo = self.repository.clone();
        let search = self.search_index.clone();
        let artifact_id = artifact.id();
        let artifact_name = artifact.name().to_string();
        let artifact_ns = artifact.namespace().to_string();
        let artifact_tier = artifact.tier();

        let save_future = {
            let repo = repo.clone();
            let artifact = artifact.clone();
            async move {
                repo.save(&artifact).await?;
                repo.store_content(&content_hash, &input.content).await?;
                Ok::<_, contextops_domain::ports::repository::RepositoryError>(())
            }
        };

        let index_future = async move {
            search
                .index(artifact_id, &artifact_name, &artifact_ns, artifact_tier, &content_str)
                .await
                .ok(); // Index failure is non-critical
        };

        // Fan-out: save and index concurrently
        let (save_result, _) = tokio::join!(save_future, index_future);

        save_result.map_err(|e| DomainError::InvalidContent {
            reason: e.to_string(),
        })?;

        // Publish domain events
        let events = artifact.take_events();
        let envelopes: Vec<_> = events
            .into_iter()
            .map(|e| e.into_envelope(artifact.id().to_string()))
            .collect();

        self.event_bus.publish(envelopes).await.ok(); // Event publishing is best-effort

        Ok(ArtifactDto::from(&artifact))
    }
}
