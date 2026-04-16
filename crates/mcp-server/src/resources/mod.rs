//! MCP resource definitions — read operations (queries).
//! Resources = read operations per MCP convention and skill2026 Rule 6.

use crate::protocol::McpResourceDefinition;

/// Returns all resource definitions for the ContextOps MCP server.
pub fn resource_definitions() -> Vec<McpResourceDefinition> {
    vec![
        McpResourceDefinition {
            uri: "contextops://artifacts".into(),
            name: "Context Artifacts".into(),
            description: "List all context artifacts in the registry".into(),
            mime_type: Some("application/json".into()),
        },
        McpResourceDefinition {
            uri: "contextops://artifacts/{artifact_id}".into(),
            name: "Context Artifact Detail".into(),
            description: "Get a specific context artifact with version history".into(),
            mime_type: Some("application/json".into()),
        },
        McpResourceDefinition {
            uri: "contextops://artifacts/{artifact_id}/content".into(),
            name: "Artifact Content".into(),
            description: "Get the raw content of a context artifact's latest version".into(),
            mime_type: Some("text/plain".into()),
        },
        McpResourceDefinition {
            uri: "contextops://resolve/{namespace}".into(),
            name: "Resolved Context".into(),
            description: "Resolve the effective context for a namespace by walking the Tier 1->2->3 inheritance chain".into(),
            mime_type: Some("application/json".into()),
        },
        McpResourceDefinition {
            uri: "contextops://search?q={query}".into(),
            name: "Search Artifacts".into(),
            description: "Full-text search across context artifacts".into(),
            mime_type: Some("application/json".into()),
        },
        McpResourceDefinition {
            uri: "contextops://pipeline/runs".into(),
            name: "Pipeline Runs".into(),
            description: "List recent pipeline runs".into(),
            mime_type: Some("application/json".into()),
        },
        McpResourceDefinition {
            uri: "contextops://pipeline/runs/{run_id}".into(),
            name: "Pipeline Run Detail".into(),
            description: "Get details of a specific pipeline run including stage results".into(),
            mime_type: Some("application/json".into()),
        },
    ]
}
