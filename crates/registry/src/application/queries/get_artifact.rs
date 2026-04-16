use std::sync::Arc;
use uuid::Uuid;

use contextops_domain::errors::DomainError;
use contextops_domain::ports::repository::ContextArtifactRepositoryPort;

use crate::application::dtos::ArtifactDetailDto;

/// Query to get a single artifact with its version history.
pub struct GetArtifactQuery {
    repository: Arc<dyn ContextArtifactRepositoryPort>,
}

impl GetArtifactQuery {
    pub fn new(repository: Arc<dyn ContextArtifactRepositoryPort>) -> Self {
        Self { repository }
    }

    pub async fn by_id(&self, id: Uuid) -> Result<ArtifactDetailDto, DomainError> {
        let artifact = self
            .repository
            .find_by_id(id)
            .await
            .map_err(|e| DomainError::InvalidContent { reason: e.to_string() })?
            .ok_or(DomainError::ArtifactNotFound { id })?;

        Ok(ArtifactDetailDto::from(&artifact))
    }

    pub async fn by_name(
        &self,
        namespace: &str,
        name: &str,
    ) -> Result<ArtifactDetailDto, DomainError> {
        let artifact = self
            .repository
            .find_by_name(namespace, name)
            .await
            .map_err(|e| DomainError::InvalidContent { reason: e.to_string() })?
            .ok_or(DomainError::InvalidContent {
                reason: format!("artifact '{name}' not found in namespace '{namespace}'"),
            })?;

        Ok(ArtifactDetailDto::from(&artifact))
    }

    /// Retrieve the raw content of a specific version.
    pub async fn content(
        &self,
        id: Uuid,
        version: Option<u64>,
    ) -> Result<Vec<u8>, DomainError> {
        let artifact = self
            .repository
            .find_by_id(id)
            .await
            .map_err(|e| DomainError::InvalidContent { reason: e.to_string() })?
            .ok_or(DomainError::ArtifactNotFound { id })?;

        let target_version = match version {
            Some(v) => artifact
                .versions()
                .iter()
                .find(|ver| ver.version() == v)
                .ok_or(DomainError::InvalidContent {
                    reason: format!("version {v} not found"),
                })?,
            None => artifact.latest_version(),
        };

        let content = self
            .repository
            .get_content(target_version.content_hash())
            .await
            .map_err(|e| DomainError::InvalidContent { reason: e.to_string() })?
            .ok_or(DomainError::InvalidContent {
                reason: "content not found in store".into(),
            })?;

        Ok(content)
    }
}
