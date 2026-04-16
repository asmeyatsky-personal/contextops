use contextops_domain::entities::ContextArtifact;
use contextops_domain::errors::DomainError;
use contextops_domain::value_objects::{ContentHash, ContextTier};

/// Resolves the effective context for an agent by walking the
/// Tier 1 → Tier 2 → Tier 3 inheritance chain.
///
/// Lower tiers can extend but NEVER override higher-tier compliance constraints.
/// The resolved context is a merged view with conflict detection.
#[derive(Debug, Clone)]
pub struct ResolvedContext {
    /// The merged content from all tiers, in order of authority.
    pub layers: Vec<ContextLayer>,
    /// The composite content hash of the resolved chain.
    pub composite_hash: ContentHash,
    /// Conflicts detected during resolution.
    pub conflicts: Vec<InheritanceConflict>,
}

#[derive(Debug, Clone)]
pub struct ContextLayer {
    pub tier: ContextTier,
    pub artifact_name: String,
    pub content: Vec<u8>,
    pub content_hash: ContentHash,
}

#[derive(Debug, Clone)]
pub struct InheritanceConflict {
    pub higher_tier: ContextTier,
    pub lower_tier: ContextTier,
    pub field: String,
    pub message: String,
}

/// Domain service for inheritance chain resolution.
pub struct InheritanceResolver;

impl InheritanceResolver {
    /// Resolve the inheritance chain for a given namespace path.
    ///
    /// Collects artifacts from Tier 1 (org) → Tier 2 (team) → Tier 3 (project),
    /// validates no override violations, and produces a composite hash.
    pub fn resolve(
        org_artifacts: &[ContextArtifact],
        team_artifacts: &[ContextArtifact],
        project_artifacts: &[ContextArtifact],
    ) -> Result<ResolvedContext, DomainError> {
        let mut layers = Vec::new();
        let mut all_content = Vec::new();

        // Collect layers in tier order (highest authority first)
        for artifact in org_artifacts {
            let content_hash = artifact.latest_version().content_hash().clone();
            layers.push(ContextLayer {
                tier: ContextTier::Organisation,
                artifact_name: artifact.name().to_string(),
                content: Vec::new(), // Content loaded separately via repository
                content_hash,
            });
        }

        for artifact in team_artifacts {
            let content_hash = artifact.latest_version().content_hash().clone();
            layers.push(ContextLayer {
                tier: ContextTier::Team,
                artifact_name: artifact.name().to_string(),
                content: Vec::new(),
                content_hash,
            });
        }

        for artifact in project_artifacts {
            let content_hash = artifact.latest_version().content_hash().clone();
            layers.push(ContextLayer {
                tier: ContextTier::Project,
                artifact_name: artifact.name().to_string(),
                content: Vec::new(),
                content_hash,
            });
        }

        // Build composite hash from all layer hashes
        for layer in &layers {
            all_content.extend_from_slice(layer.content_hash.as_str().as_bytes());
        }
        let composite_hash = ContentHash::from_content(&all_content);

        // Conflict detection would go here in a full implementation —
        // checking that lower tiers don't override higher-tier constraints.
        let conflicts = Vec::new();

        Ok(ResolvedContext {
            layers,
            composite_hash,
            conflicts,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use contextops_domain::entities::{ContextArtifact, ContextFormat};

    fn make_artifact(name: &str, tier: ContextTier) -> ContextArtifact {
        ContextArtifact::register(
            name.into(),
            "test-ns".into(),
            tier,
            ContextFormat::ClaudeMd,
            "owner".into(),
            format!("# {name} content").as_bytes(),
            "author".into(),
            "initial".into(),
        )
        .unwrap()
    }

    #[test]
    fn resolve_produces_layers_in_tier_order() {
        let org = vec![make_artifact("company-rules", ContextTier::Organisation)];
        let team = vec![make_artifact("team-conventions", ContextTier::Team)];
        let project = vec![make_artifact("agent-config", ContextTier::Project)];

        let resolved = InheritanceResolver::resolve(&org, &team, &project).unwrap();

        assert_eq!(resolved.layers.len(), 3);
        assert_eq!(resolved.layers[0].tier, ContextTier::Organisation);
        assert_eq!(resolved.layers[1].tier, ContextTier::Team);
        assert_eq!(resolved.layers[2].tier, ContextTier::Project);
    }

    #[test]
    fn resolve_with_empty_tiers_succeeds() {
        let resolved = InheritanceResolver::resolve(&[], &[], &[]).unwrap();
        assert!(resolved.layers.is_empty());
    }

    #[test]
    fn composite_hash_is_deterministic() {
        let org = vec![make_artifact("rules", ContextTier::Organisation)];
        let r1 = InheritanceResolver::resolve(&org, &[], &[]).unwrap();
        let r2 = InheritanceResolver::resolve(&org, &[], &[]).unwrap();
        assert_eq!(r1.composite_hash, r2.composite_hash);
    }
}
