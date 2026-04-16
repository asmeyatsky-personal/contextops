use std::sync::Arc;

use contextops_common::adapters::{
    InMemoryArtifactRepository, InMemoryEventBus, InMemorySearchIndex,
    PassthroughSchemaValidator,
};
use contextops_domain::ports::event_bus::EventBusPort;
use contextops_domain::ports::repository::ContextArtifactRepositoryPort;
use contextops_domain::ports::schema_validator::SchemaValidatorPort;
use contextops_domain::ports::search_index::SearchIndexPort;

use crate::application::commands::{
    CreateVersionCommand, DeprecateArtifactCommand, RegisterArtifactCommand,
};
use crate::application::queries::{
    GetArtifactQuery, ListArtifactsQuery, ResolveContextQuery, SearchArtifactsQuery,
};

/// Dependency injection container for the Context Registry bounded context.
/// Composition root — wires implementations to ports.
///
/// Production would swap in Firestore/GCS adapters, Pub/Sub event bus, etc.
pub struct RegistryContainer {
    // Ports
    pub repository: Arc<dyn ContextArtifactRepositoryPort>,
    pub validator: Arc<dyn SchemaValidatorPort>,
    pub search_index: Arc<dyn SearchIndexPort>,
    pub event_bus: Arc<dyn EventBusPort>,

    // Commands
    pub register_artifact: RegisterArtifactCommand,
    pub create_version: CreateVersionCommand,
    pub deprecate_artifact: DeprecateArtifactCommand,

    // Queries
    pub get_artifact: GetArtifactQuery,
    pub list_artifacts: ListArtifactsQuery,
    pub search_artifacts: SearchArtifactsQuery,
    pub resolve_context: ResolveContextQuery,
}

impl RegistryContainer {
    /// Create a container with in-memory adapters (development/testing).
    pub fn in_memory() -> Self {
        let repository: Arc<dyn ContextArtifactRepositoryPort> =
            Arc::new(InMemoryArtifactRepository::new());
        let validator: Arc<dyn SchemaValidatorPort> =
            Arc::new(PassthroughSchemaValidator::new());
        let search_index: Arc<dyn SearchIndexPort> =
            Arc::new(InMemorySearchIndex::new());
        let event_bus: Arc<dyn EventBusPort> =
            Arc::new(InMemoryEventBus::new());

        let register_artifact = RegisterArtifactCommand::new(
            repository.clone(),
            validator.clone(),
            search_index.clone(),
            event_bus.clone(),
        );
        let create_version = CreateVersionCommand::new(
            repository.clone(),
            validator.clone(),
            search_index.clone(),
            event_bus.clone(),
        );
        let deprecate_artifact = DeprecateArtifactCommand::new(
            repository.clone(),
            event_bus.clone(),
        );
        let get_artifact = GetArtifactQuery::new(repository.clone());
        let list_artifacts = ListArtifactsQuery::new(repository.clone());
        let search_artifacts = SearchArtifactsQuery::new(search_index.clone());
        let resolve_context = ResolveContextQuery::new(repository.clone());

        Self {
            repository,
            validator,
            search_index,
            event_bus,
            register_artifact,
            create_version,
            deprecate_artifact,
            get_artifact,
            list_artifacts,
            search_artifacts,
            resolve_context,
        }
    }
}
