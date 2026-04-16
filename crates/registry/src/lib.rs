/// M1 — Context Registry Bounded Context
///
/// Architectural Intent:
/// - Canonical source of truth for all versioned context artifacts
/// - Content-addressable storage with SHA-256 digest pinning
/// - Three-tier hierarchy with inheritance chain resolution
/// - VAID binding for agent↔context verification
///
/// MCP Integration:
/// - Exposed as 'context-registry' MCP server
/// - Tools: register_artifact, create_version, promote, deprecate
/// - Resources: artifact://{id}, artifact://list, artifact://search
///
/// Parallelization:
/// - Schema validation and search indexing run concurrently on register
/// - Inheritance chain resolution parallelizes tier lookups

pub mod application;
pub mod domain;
pub mod infrastructure;
