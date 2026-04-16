use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use contextops_domain::entities::{ContextArtifact, ContextFormat, ContextVersion};
use contextops_domain::value_objects::ContextTier;

/// Data Transfer Object for context artifact responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactDto {
    pub id: Uuid,
    pub name: String,
    pub namespace: String,
    pub tier: ContextTier,
    pub format: ContextFormat,
    pub owner: String,
    pub current_version: u64,
    pub deprecated: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub latest_content_hash: String,
    pub has_active_vaid: bool,
}

impl From<&ContextArtifact> for ArtifactDto {
    fn from(a: &ContextArtifact) -> Self {
        Self {
            id: a.id(),
            name: a.name().to_string(),
            namespace: a.namespace().to_string(),
            tier: a.tier(),
            format: a.format(),
            owner: a.owner().to_string(),
            current_version: a.current_version(),
            deprecated: a.is_deprecated(),
            created_at: a.created_at(),
            updated_at: a.updated_at(),
            latest_content_hash: a.latest_version().content_hash().as_str().to_string(),
            has_active_vaid: a.active_vaid().is_some_and(|v| v.is_valid()),
        }
    }
}

/// Data Transfer Object for version responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionDto {
    pub version: u64,
    pub content_hash: String,
    pub content_size_bytes: u64,
    pub author: String,
    pub message: String,
    pub created_at: DateTime<Utc>,
}

impl From<&ContextVersion> for VersionDto {
    fn from(v: &ContextVersion) -> Self {
        Self {
            version: v.version(),
            content_hash: v.content_hash().as_str().to_string(),
            content_size_bytes: v.content_size_bytes(),
            author: v.author().to_string(),
            message: v.message().to_string(),
            created_at: v.created_at(),
        }
    }
}

/// Detailed artifact DTO including version history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactDetailDto {
    #[serde(flatten)]
    pub artifact: ArtifactDto,
    pub versions: Vec<VersionDto>,
}

impl From<&ContextArtifact> for ArtifactDetailDto {
    fn from(a: &ContextArtifact) -> Self {
        Self {
            artifact: ArtifactDto::from(a),
            versions: a.versions().iter().map(VersionDto::from).collect(),
        }
    }
}
