use std::sync::Arc;

use contextops_domain::errors::DomainError;
use contextops_domain::ports::repository::ContextArtifactRepositoryPort;
use contextops_domain::value_objects::ContextTier;

use crate::application::dtos::ArtifactDto;

/// Query to list context artifacts with filtering.
pub struct ListArtifactsQuery {
    repository: Arc<dyn ContextArtifactRepositoryPort>,
}

impl ListArtifactsQuery {
    pub fn new(repository: Arc<dyn ContextArtifactRepositoryPort>) -> Self {
        Self { repository }
    }

    pub async fn all(
        &self,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<ArtifactDto>, DomainError> {
        let artifacts = self
            .repository
            .list_all(offset, limit)
            .await
            .map_err(|e| DomainError::InvalidContent { reason: e.to_string() })?;

        Ok(artifacts.iter().map(ArtifactDto::from).collect())
    }

    pub async fn by_tier(&self, tier: ContextTier) -> Result<Vec<ArtifactDto>, DomainError> {
        let artifacts = self
            .repository
            .list_by_tier(tier)
            .await
            .map_err(|e| DomainError::InvalidContent { reason: e.to_string() })?;

        Ok(artifacts.iter().map(ArtifactDto::from).collect())
    }

    pub async fn by_namespace(&self, namespace: &str) -> Result<Vec<ArtifactDto>, DomainError> {
        let artifacts = self
            .repository
            .list_by_namespace(namespace)
            .await
            .map_err(|e| DomainError::InvalidContent { reason: e.to_string() })?;

        Ok(artifacts.iter().map(ArtifactDto::from).collect())
    }

    pub async fn count(&self) -> Result<usize, DomainError> {
        self.repository
            .count()
            .await
            .map_err(|e| DomainError::InvalidContent { reason: e.to_string() })
    }
}
