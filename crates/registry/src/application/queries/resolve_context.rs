use std::sync::Arc;

use contextops_domain::errors::DomainError;
use contextops_domain::ports::repository::ContextArtifactRepositoryPort;
use contextops_domain::value_objects::ContextTier;

use crate::domain::services::{InheritanceResolver, ResolvedContext};

/// Query to resolve the effective context for an agent by walking
/// the Tier 1 → Tier 2 → Tier 3 inheritance chain.
///
/// Parallelization: tier lookups fan out concurrently.
pub struct ResolveContextQuery {
    repository: Arc<dyn ContextArtifactRepositoryPort>,
}

impl ResolveContextQuery {
    pub fn new(repository: Arc<dyn ContextArtifactRepositoryPort>) -> Self {
        Self { repository }
    }

    pub async fn resolve(&self, namespace: &str) -> Result<ResolvedContext, DomainError> {
        // Fan-out: fetch all three tiers concurrently
        let repo = self.repository.clone();
        let repo2 = self.repository.clone();
        let repo3 = self.repository.clone();

        let ns_team = namespace.to_string();
        let ns_project = namespace.to_string();

        let (org_result, team_result, project_result) = tokio::join!(
            async move { repo.list_by_tier(ContextTier::Organisation).await },
            async move {
                repo2.list_by_tier(ContextTier::Team).await.map(|artifacts| {
                    artifacts
                        .into_iter()
                        .filter(|a| a.namespace() == ns_team || a.namespace().starts_with(&format!("{ns_team}/")))
                        .collect::<Vec<_>>()
                })
            },
            async move {
                repo3.list_by_tier(ContextTier::Project).await.map(|artifacts| {
                    artifacts
                        .into_iter()
                        .filter(|a| a.namespace() == ns_project)
                        .collect::<Vec<_>>()
                })
            },
        );

        let org_artifacts = org_result
            .map_err(|e| DomainError::InheritanceResolutionFailed { reason: e.to_string() })?;
        let team_artifacts = team_result
            .map_err(|e| DomainError::InheritanceResolutionFailed { reason: e.to_string() })?;
        let project_artifacts = project_result
            .map_err(|e| DomainError::InheritanceResolutionFailed { reason: e.to_string() })?;

        InheritanceResolver::resolve(&org_artifacts, &team_artifacts, &project_artifacts)
    }
}
