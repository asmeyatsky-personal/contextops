//! MCP tool definitions — write operations (commands).
//! Tools = write operations per MCP convention and skill2026 Rule 6.

use crate::protocol::McpToolDefinition;

/// Returns all tool definitions for the ContextOps MCP server.
pub fn tool_definitions() -> Vec<McpToolDefinition> {
    vec![
        McpToolDefinition {
            name: "register_artifact".into(),
            description: "Register a new context artifact in the Context Registry. Creates the artifact with its initial version.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Unique name for the context artifact within its namespace"
                    },
                    "namespace": {
                        "type": "string",
                        "description": "Hierarchical namespace (e.g., 'org/finance/billing')"
                    },
                    "tier": {
                        "type": "string",
                        "enum": ["organisation", "team", "project"],
                        "description": "Context tier in the hierarchy"
                    },
                    "format": {
                        "type": "string",
                        "enum": ["claude-md", "json", "yaml", "plain-text", "prompt-template"],
                        "description": "Format of the context content"
                    },
                    "owner": {
                        "type": "string",
                        "description": "Owner of this context artifact"
                    },
                    "content": {
                        "type": "string",
                        "description": "The context content"
                    },
                    "author": {
                        "type": "string",
                        "description": "Author of this version"
                    },
                    "message": {
                        "type": "string",
                        "description": "Commit message describing this version"
                    }
                },
                "required": ["name", "namespace", "tier", "format", "owner", "content", "author", "message"]
            }),
        },
        McpToolDefinition {
            name: "create_version".into(),
            description: "Create a new version of an existing context artifact. Content-addressable — same content produces same hash.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "artifact_id": {
                        "type": "string",
                        "description": "UUID of the artifact to version"
                    },
                    "content": {
                        "type": "string",
                        "description": "New content for this version"
                    },
                    "author": {
                        "type": "string",
                        "description": "Author of this version"
                    },
                    "message": {
                        "type": "string",
                        "description": "Change description"
                    }
                },
                "required": ["artifact_id", "content", "author", "message"]
            }),
        },
        McpToolDefinition {
            name: "deprecate_artifact".into(),
            description: "Mark a context artifact as deprecated. Deprecated artifacts cannot have new versions created.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "artifact_id": {
                        "type": "string",
                        "description": "UUID of the artifact to deprecate"
                    },
                    "reason": {
                        "type": "string",
                        "description": "Reason for deprecation"
                    }
                },
                "required": ["artifact_id", "reason"]
            }),
        },
        McpToolDefinition {
            name: "run_pipeline".into(),
            description: "Run the context pipeline against an artifact. Executes validate, blast radius, regression test, security scan, and promotion stages.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "artifact_id": {
                        "type": "string",
                        "description": "UUID of the artifact to run the pipeline against"
                    },
                    "pipeline_id": {
                        "type": "string",
                        "description": "UUID of the pipeline to run (optional, defaults to standard pipeline)"
                    },
                    "trigger": {
                        "type": "string",
                        "description": "What triggered this pipeline run (e.g., 'manual', 'webhook', 'schedule')"
                    }
                },
                "required": ["artifact_id"]
            }),
        },
    ]
}
