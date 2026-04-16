/// ContextOps™ Common Infrastructure Utilities
///
/// Architectural Intent:
/// - Shared infrastructure code used by multiple bounded contexts
/// - In-memory adapter implementations for development and testing
/// - Common configuration and DI utilities
///
/// This crate sits in the infrastructure layer — it depends on domain
/// but is never depended on by domain.

pub mod adapters;
