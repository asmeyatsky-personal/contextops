/// M2 — Context Pipeline Bounded Context
///
/// Architectural Intent:
/// - DAG-based CI/CD pipeline for context artifacts
/// - Stages: Validate → Blast Radius → Regression Test → Security Scan → Promote → Rollback
/// - Parallelism-first: independent stages execute concurrently
/// - Harness-native pipeline template compatibility
///
/// MCP Integration:
/// - Exposed as 'context-pipeline' MCP server
/// - Tools: run_pipeline, trigger_rollback, approve_promotion
/// - Resources: pipeline://{id}/status, pipeline://runs
///
/// Key Design Decisions:
/// 1. Pipelines are DAGs — stages have explicit dependencies
/// 2. Each stage is behind a port (StageExecutor) — swappable implementations
/// 3. Blast radius computation is a first-class pipeline concern
/// 4. Rollback is automatic on drift threshold breach

pub mod application;
pub mod domain;
pub mod infrastructure;
