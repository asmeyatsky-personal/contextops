//! MCP server implementation — handles JSON-RPC requests and routes to
//! the appropriate tool/resource handler.

use std::sync::Arc;

use contextops_domain::entities::ContextFormat;
use contextops_domain::value_objects::ContextTier;
use contextops_registry::application::commands::{
    CreateVersionCommand, DeprecateArtifactCommand, RegisterArtifactCommand,
};
use contextops_registry::application::queries::{
    GetArtifactQuery, ListArtifactsQuery, ResolveContextQuery, SearchArtifactsQuery,
};
use contextops_pipeline::application::commands::RunPipelineCommand;
use contextops_pipeline::application::queries::GetPipelineRunQuery;

use crate::protocol::{
    JsonRpcRequest, JsonRpcResponse, McpCapabilities, McpServerInfo,
};
use crate::resources::resource_definitions;
use crate::tools::tool_definitions;

/// The ContextOps MCP server.
pub struct ContextOpsMcpServer {
    // Registry commands
    register_artifact: Arc<RegisterArtifactCommand>,
    create_version: Arc<CreateVersionCommand>,
    deprecate_artifact: Arc<DeprecateArtifactCommand>,

    // Registry queries
    get_artifact: Arc<GetArtifactQuery>,
    list_artifacts: Arc<ListArtifactsQuery>,
    search_artifacts: Arc<SearchArtifactsQuery>,
    resolve_context: Arc<ResolveContextQuery>,

    // Pipeline
    run_pipeline: Arc<RunPipelineCommand>,
    get_pipeline_run: Arc<GetPipelineRunQuery>,
}

impl ContextOpsMcpServer {
    pub fn new(
        register_artifact: RegisterArtifactCommand,
        create_version: CreateVersionCommand,
        deprecate_artifact: DeprecateArtifactCommand,
        get_artifact: GetArtifactQuery,
        list_artifacts: ListArtifactsQuery,
        search_artifacts: SearchArtifactsQuery,
        resolve_context: ResolveContextQuery,
        run_pipeline: RunPipelineCommand,
        get_pipeline_run: GetPipelineRunQuery,
    ) -> Self {
        Self {
            register_artifact: Arc::new(register_artifact),
            create_version: Arc::new(create_version),
            deprecate_artifact: Arc::new(deprecate_artifact),
            get_artifact: Arc::new(get_artifact),
            list_artifacts: Arc::new(list_artifacts),
            search_artifacts: Arc::new(search_artifacts),
            resolve_context: Arc::new(resolve_context),
            run_pipeline: Arc::new(run_pipeline),
            get_pipeline_run: Arc::new(get_pipeline_run),
        }
    }

    /// Handle a JSON-RPC request and return a response.
    pub async fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        match request.method.as_str() {
            "initialize" => self.handle_initialize(request.id),
            "tools/list" => self.handle_list_tools(request.id),
            "tools/call" => self.handle_tool_call(request.id, request.params).await,
            "resources/list" => self.handle_list_resources(request.id),
            "resources/read" => self.handle_resource_read(request.id, request.params).await,
            _ => JsonRpcResponse::error(
                request.id,
                -32601,
                format!("Method not found: {}", request.method),
            ),
        }
    }

    fn handle_initialize(&self, id: serde_json::Value) -> JsonRpcResponse {
        JsonRpcResponse::success(
            id,
            serde_json::json!({
                "protocolVersion": "2024-11-05",
                "serverInfo": McpServerInfo {
                    name: "contextops".into(),
                    version: env!("CARGO_PKG_VERSION").into(),
                },
                "capabilities": McpCapabilities {
                    tools: Some(serde_json::json!({})),
                    resources: Some(serde_json::json!({})),
                    prompts: Some(serde_json::json!({})),
                },
            }),
        )
    }

    fn handle_list_tools(&self, id: serde_json::Value) -> JsonRpcResponse {
        JsonRpcResponse::success(
            id,
            serde_json::json!({ "tools": tool_definitions() }),
        )
    }

    fn handle_list_resources(&self, id: serde_json::Value) -> JsonRpcResponse {
        JsonRpcResponse::success(
            id,
            serde_json::json!({ "resources": resource_definitions() }),
        )
    }

    async fn handle_tool_call(
        &self,
        id: serde_json::Value,
        params: serde_json::Value,
    ) -> JsonRpcResponse {
        let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

        match tool_name {
            "register_artifact" => self.call_register_artifact(id, arguments).await,
            "create_version" => self.call_create_version(id, arguments).await,
            "deprecate_artifact" => self.call_deprecate_artifact(id, arguments).await,
            "run_pipeline" => self.call_run_pipeline(id, arguments).await,
            _ => JsonRpcResponse::error(id, -32602, format!("Unknown tool: {tool_name}")),
        }
    }

    async fn call_register_artifact(
        &self,
        id: serde_json::Value,
        args: serde_json::Value,
    ) -> JsonRpcResponse {
        let name = args["name"].as_str().unwrap_or_default().to_string();
        let namespace = args["namespace"].as_str().unwrap_or_default().to_string();
        let tier = match args["tier"].as_str().unwrap_or("project") {
            "organisation" => ContextTier::Organisation,
            "team" => ContextTier::Team,
            _ => ContextTier::Project,
        };
        let format = match args["format"].as_str().unwrap_or("claude-md") {
            "json" => ContextFormat::Json,
            "yaml" => ContextFormat::Yaml,
            "plain-text" => ContextFormat::PlainText,
            "prompt-template" => ContextFormat::PromptTemplate,
            _ => ContextFormat::ClaudeMd,
        };
        let owner = args["owner"].as_str().unwrap_or_default().to_string();
        let content = args["content"].as_str().unwrap_or_default().as_bytes().to_vec();
        let author = args["author"].as_str().unwrap_or_default().to_string();
        let message = args["message"].as_str().unwrap_or_default().to_string();

        let input = contextops_registry::application::commands::register_artifact::RegisterArtifactInput {
            name, namespace, tier, format, owner, content, author, message,
        };

        match self.register_artifact.execute(input).await {
            Ok(dto) => JsonRpcResponse::success(
                id,
                serde_json::json!({
                    "content": [{ "type": "text", "text": serde_json::to_string_pretty(&dto).unwrap() }]
                }),
            ),
            Err(e) => JsonRpcResponse::error(id, -32000, e.to_string()),
        }
    }

    async fn call_create_version(
        &self,
        id: serde_json::Value,
        args: serde_json::Value,
    ) -> JsonRpcResponse {
        let artifact_id = match args["artifact_id"].as_str().and_then(|s| s.parse().ok()) {
            Some(id) => id,
            None => return JsonRpcResponse::error(id, -32602, "Invalid artifact_id".into()),
        };
        let content = args["content"].as_str().unwrap_or_default().as_bytes().to_vec();
        let author = args["author"].as_str().unwrap_or_default().to_string();
        let message = args["message"].as_str().unwrap_or_default().to_string();

        let input = contextops_registry::application::commands::create_version::CreateVersionInput {
            artifact_id, content, author, message, commit_sha: None,
        };

        match self.create_version.execute(input).await {
            Ok(dto) => JsonRpcResponse::success(
                id,
                serde_json::json!({
                    "content": [{ "type": "text", "text": serde_json::to_string_pretty(&dto).unwrap() }]
                }),
            ),
            Err(e) => JsonRpcResponse::error(id, -32000, e.to_string()),
        }
    }

    async fn call_deprecate_artifact(
        &self,
        id: serde_json::Value,
        args: serde_json::Value,
    ) -> JsonRpcResponse {
        let artifact_id = match args["artifact_id"].as_str().and_then(|s| s.parse().ok()) {
            Some(id) => id,
            None => return JsonRpcResponse::error(id, -32602, "Invalid artifact_id".into()),
        };
        let reason = args["reason"].as_str().unwrap_or_default().to_string();

        let input = contextops_registry::application::commands::deprecate_artifact::DeprecateArtifactInput {
            artifact_id, reason,
        };

        match self.deprecate_artifact.execute(input).await {
            Ok(dto) => JsonRpcResponse::success(
                id,
                serde_json::json!({
                    "content": [{ "type": "text", "text": serde_json::to_string_pretty(&dto).unwrap() }]
                }),
            ),
            Err(e) => JsonRpcResponse::error(id, -32000, e.to_string()),
        }
    }

    async fn call_run_pipeline(
        &self,
        id: serde_json::Value,
        args: serde_json::Value,
    ) -> JsonRpcResponse {
        let artifact_id = match args["artifact_id"].as_str().and_then(|s| s.parse().ok()) {
            Some(id) => id,
            None => return JsonRpcResponse::error(id, -32602, "Invalid artifact_id".into()),
        };
        let pipeline_id = args["pipeline_id"]
            .as_str()
            .and_then(|s| s.parse().ok());
        let trigger = args["trigger"]
            .as_str()
            .unwrap_or("mcp")
            .to_string();

        let input = contextops_pipeline::application::commands::RunPipelineInput {
            pipeline_id, artifact_id, trigger,
        };

        match self.run_pipeline.execute(input).await {
            Ok(dto) => JsonRpcResponse::success(
                id,
                serde_json::json!({
                    "content": [{ "type": "text", "text": serde_json::to_string_pretty(&dto).unwrap() }]
                }),
            ),
            Err(e) => JsonRpcResponse::error(id, -32000, e.to_string()),
        }
    }

    async fn handle_resource_read(
        &self,
        id: serde_json::Value,
        params: serde_json::Value,
    ) -> JsonRpcResponse {
        let uri = params.get("uri").and_then(|v| v.as_str()).unwrap_or("");

        if uri == "contextops://artifacts" {
            match self.list_artifacts.all(0, 100).await {
                Ok(artifacts) => JsonRpcResponse::success(
                    id,
                    serde_json::json!({
                        "contents": [{
                            "uri": uri,
                            "mimeType": "application/json",
                            "text": serde_json::to_string_pretty(&artifacts).unwrap()
                        }]
                    }),
                ),
                Err(e) => JsonRpcResponse::error(id, -32000, e.to_string()),
            }
        } else if let Some(artifact_id_str) = uri.strip_prefix("contextops://artifacts/") {
            let artifact_id_str = artifact_id_str.trim_end_matches("/content");
            match artifact_id_str.parse() {
                Ok(artifact_id) => {
                    if uri.ends_with("/content") {
                        match self.get_artifact.content(artifact_id, None).await {
                            Ok(content) => JsonRpcResponse::success(
                                id,
                                serde_json::json!({
                                    "contents": [{
                                        "uri": uri,
                                        "mimeType": "text/plain",
                                        "text": String::from_utf8_lossy(&content)
                                    }]
                                }),
                            ),
                            Err(e) => JsonRpcResponse::error(id, -32000, e.to_string()),
                        }
                    } else {
                        match self.get_artifact.by_id(artifact_id).await {
                            Ok(detail) => JsonRpcResponse::success(
                                id,
                                serde_json::json!({
                                    "contents": [{
                                        "uri": uri,
                                        "mimeType": "application/json",
                                        "text": serde_json::to_string_pretty(&detail).unwrap()
                                    }]
                                }),
                            ),
                            Err(e) => JsonRpcResponse::error(id, -32000, e.to_string()),
                        }
                    }
                }
                Err(_) => JsonRpcResponse::error(id, -32602, "Invalid artifact ID".into()),
            }
        } else if let Some(namespace) = uri.strip_prefix("contextops://resolve/") {
            match self.resolve_context.resolve(namespace).await {
                Ok(resolved) => JsonRpcResponse::success(
                    id,
                    serde_json::json!({
                        "contents": [{
                            "uri": uri,
                            "mimeType": "application/json",
                            "text": serde_json::json!({
                                "layers": resolved.layers.len(),
                                "composite_hash": resolved.composite_hash.as_str(),
                                "conflicts": resolved.conflicts.len(),
                            }).to_string()
                        }]
                    }),
                ),
                Err(e) => JsonRpcResponse::error(id, -32000, e.to_string()),
            }
        } else if uri == "contextops://pipeline/runs" {
            match self.get_pipeline_run.recent(20).await {
                Ok(runs) => JsonRpcResponse::success(
                    id,
                    serde_json::json!({
                        "contents": [{
                            "uri": uri,
                            "mimeType": "application/json",
                            "text": serde_json::to_string_pretty(&runs).unwrap()
                        }]
                    }),
                ),
                Err(e) => JsonRpcResponse::error(id, -32000, e.to_string()),
            }
        } else if let Some(run_id_str) = uri.strip_prefix("contextops://pipeline/runs/") {
            match run_id_str.parse() {
                Ok(run_id) => match self.get_pipeline_run.by_id(run_id).await {
                    Ok(detail) => JsonRpcResponse::success(
                        id,
                        serde_json::json!({
                            "contents": [{
                                "uri": uri,
                                "mimeType": "application/json",
                                "text": serde_json::to_string_pretty(&detail).unwrap()
                            }]
                        }),
                    ),
                    Err(e) => JsonRpcResponse::error(id, -32000, e.to_string()),
                },
                Err(_) => JsonRpcResponse::error(id, -32602, "Invalid run ID".into()),
            }
        } else {
            JsonRpcResponse::error(id, -32602, format!("Unknown resource URI: {uri}"))
        }
    }
}
