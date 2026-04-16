/// ContextOps™ MCP Server
///
/// Architectural Intent:
/// - MCP protocol layer for all agent-facing interactions
/// - Tools (write operations): register_artifact, create_version, deprecate, run_pipeline
/// - Resources (read operations): artifact://{id}, artifact://list, pipeline://runs
/// - Prompts (interaction patterns): context_summary, audit_report
///
/// All modules expose MCP servers. Agents interact with ContextOps
/// at runtime exclusively via MCP — zero SDK coupling.
///
/// JSON-RPC 2.0 transport over stdio.

pub mod protocol;
pub mod server;
pub mod tools;
pub mod resources;
