/// ContextOps™ Shared Domain Primitives
///
/// Architectural Intent:
/// - Defines the ubiquitous language for the ContextOps™ platform
/// - Contains value objects, domain event base types, error types,
///   and port trait definitions shared across bounded contexts
/// - ZERO infrastructure dependencies — this crate is pure domain logic
///
/// Bounded Context Alignment:
/// - Used by: ContextRegistry, ContextPipeline, DriftDetection,
///   SecurityGateway, FeedbackLoop, GovernanceConsole
/// - Provides the shared kernel that all contexts agree on

pub mod entities;
pub mod errors;
pub mod events;
pub mod ports;
pub mod value_objects;
