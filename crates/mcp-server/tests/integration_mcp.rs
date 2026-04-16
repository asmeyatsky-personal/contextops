/// Integration tests for the ContextOps MCP server.
///
/// Tests protocol compliance, tool call routing, resource reading,
/// error handling, and end-to-end tool invocation flows.

use contextops_mcp_server::protocol::JsonRpcRequest;
use contextops_mcp_server::server::ContextOpsMcpServer;
use contextops_pipeline::infrastructure::container::PipelineContainer;
use contextops_registry::infrastructure::container::RegistryContainer;

fn setup() -> ContextOpsMcpServer {
    let registry = RegistryContainer::in_memory();
    let pipeline = PipelineContainer::in_memory(registry.repository.clone());
    ContextOpsMcpServer::new(
        registry.register_artifact,
        registry.create_version,
        registry.deprecate_artifact,
        registry.get_artifact,
        registry.list_artifacts,
        registry.search_artifacts,
        registry.resolve_context,
        pipeline.run_pipeline,
        pipeline.get_pipeline_run,
    )
}

fn make_request(method: &str, params: serde_json::Value) -> JsonRpcRequest {
    JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: serde_json::json!(1),
        method: method.into(),
        params,
    }
}

// --- Protocol Tests ---

#[tokio::test]
async fn initialize_returns_server_info() {
    let server = setup();
    let req = make_request("initialize", serde_json::json!({}));
    let resp = server.handle_request(req).await;

    assert!(resp.error.is_none());
    let result = resp.result.unwrap();
    assert_eq!(result["protocolVersion"], "2024-11-05");
    assert_eq!(result["serverInfo"]["name"], "contextops");
    assert!(result["capabilities"]["tools"].is_object());
    assert!(result["capabilities"]["resources"].is_object());
}

#[tokio::test]
async fn unknown_method_returns_error() {
    let server = setup();
    let req = make_request("nonexistent/method", serde_json::json!({}));
    let resp = server.handle_request(req).await;

    assert!(resp.error.is_some());
    assert_eq!(resp.error.unwrap().code, -32601);
}

// --- Tools/List Tests ---

#[tokio::test]
async fn tools_list_returns_all_tools() {
    let server = setup();
    let req = make_request("tools/list", serde_json::json!({}));
    let resp = server.handle_request(req).await;

    assert!(resp.error.is_none());
    let tools = resp.result.unwrap()["tools"].as_array().unwrap().clone();
    let tool_names: Vec<&str> = tools
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();

    assert!(tool_names.contains(&"register_artifact"));
    assert!(tool_names.contains(&"create_version"));
    assert!(tool_names.contains(&"deprecate_artifact"));
    assert!(tool_names.contains(&"run_pipeline"));
    assert_eq!(tools.len(), 4);
}

#[tokio::test]
async fn tools_have_input_schemas() {
    let server = setup();
    let req = make_request("tools/list", serde_json::json!({}));
    let resp = server.handle_request(req).await;

    let tools = resp.result.unwrap()["tools"].as_array().unwrap().clone();
    for tool in &tools {
        assert!(tool["inputSchema"].is_object(), "tool {} missing inputSchema", tool["name"]);
        assert!(tool["description"].is_string(), "tool {} missing description", tool["name"]);
    }
}

// --- Resources/List Tests ---

#[tokio::test]
async fn resources_list_returns_all_resources() {
    let server = setup();
    let req = make_request("resources/list", serde_json::json!({}));
    let resp = server.handle_request(req).await;

    assert!(resp.error.is_none());
    let resources = resp.result.unwrap()["resources"].as_array().unwrap().clone();
    assert!(resources.len() >= 6);
}

// --- Tool Call Tests ---

#[tokio::test]
async fn register_artifact_via_tool_call() {
    let server = setup();
    let req = make_request(
        "tools/call",
        serde_json::json!({
            "name": "register_artifact",
            "arguments": {
                "name": "test-context",
                "namespace": "org/test",
                "tier": "project",
                "format": "claude-md",
                "owner": "test-owner",
                "content": "# Test\n\nThis is a test context file.",
                "author": "tester",
                "message": "initial version"
            }
        }),
    );

    let resp = server.handle_request(req).await;
    assert!(resp.error.is_none(), "register_artifact failed: {:?}", resp.error);

    let result = resp.result.unwrap();
    let content_text = result["content"][0]["text"].as_str().unwrap();
    let artifact: serde_json::Value = serde_json::from_str(content_text).unwrap();
    assert_eq!(artifact["name"], "test-context");
    assert_eq!(artifact["current_version"], 1);
}

#[tokio::test]
async fn register_and_then_create_version() {
    let server = setup();

    // Register
    let register_req = make_request(
        "tools/call",
        serde_json::json!({
            "name": "register_artifact",
            "arguments": {
                "name": "versioned-context",
                "namespace": "org/test",
                "tier": "team",
                "format": "claude-md",
                "owner": "owner",
                "content": "# V1\n\nFirst version content.",
                "author": "alice",
                "message": "v1"
            }
        }),
    );
    let resp = server.handle_request(register_req).await;
    let artifact: serde_json::Value =
        serde_json::from_str(resp.result.unwrap()["content"][0]["text"].as_str().unwrap()).unwrap();
    let artifact_id = artifact["id"].as_str().unwrap();

    // Create version
    let version_req = make_request(
        "tools/call",
        serde_json::json!({
            "name": "create_version",
            "arguments": {
                "artifact_id": artifact_id,
                "content": "# V2\n\nUpdated version content.",
                "author": "bob",
                "message": "v2 update"
            }
        }),
    );
    let resp = server.handle_request(version_req).await;
    assert!(resp.error.is_none(), "create_version failed: {:?}", resp.error);

    let updated: serde_json::Value =
        serde_json::from_str(resp.result.unwrap()["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(updated["current_version"], 2);
}

#[tokio::test]
async fn register_and_deprecate() {
    let server = setup();

    // Register
    let resp = server
        .handle_request(make_request(
            "tools/call",
            serde_json::json!({
                "name": "register_artifact",
                "arguments": {
                    "name": "to-deprecate",
                    "namespace": "org/test",
                    "tier": "project",
                    "format": "plain-text",
                    "owner": "owner",
                    "content": "Will be deprecated soon.",
                    "author": "alice",
                    "message": "temp"
                }
            }),
        ))
        .await;
    let artifact: serde_json::Value =
        serde_json::from_str(resp.result.unwrap()["content"][0]["text"].as_str().unwrap()).unwrap();
    let artifact_id = artifact["id"].as_str().unwrap();

    // Deprecate
    let resp = server
        .handle_request(make_request(
            "tools/call",
            serde_json::json!({
                "name": "deprecate_artifact",
                "arguments": {
                    "artifact_id": artifact_id,
                    "reason": "replaced by new context"
                }
            }),
        ))
        .await;
    assert!(resp.error.is_none());

    let deprecated: serde_json::Value =
        serde_json::from_str(resp.result.unwrap()["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(deprecated["deprecated"], true);
}

#[tokio::test]
async fn unknown_tool_returns_error() {
    let server = setup();
    let req = make_request(
        "tools/call",
        serde_json::json!({
            "name": "nonexistent_tool",
            "arguments": {}
        }),
    );
    let resp = server.handle_request(req).await;
    assert!(resp.error.is_some());
    assert_eq!(resp.error.unwrap().code, -32602);
}

#[tokio::test]
async fn invalid_artifact_id_returns_error() {
    let server = setup();
    let req = make_request(
        "tools/call",
        serde_json::json!({
            "name": "create_version",
            "arguments": {
                "artifact_id": "not-a-uuid",
                "content": "test",
                "author": "a",
                "message": "m"
            }
        }),
    );
    let resp = server.handle_request(req).await;
    assert!(resp.error.is_some());
}

// --- Resource Read Tests ---

#[tokio::test]
async fn read_artifacts_list_empty() {
    let server = setup();
    let req = make_request(
        "resources/read",
        serde_json::json!({"uri": "contextops://artifacts"}),
    );
    let resp = server.handle_request(req).await;
    assert!(resp.error.is_none());

    let contents = &resp.result.unwrap()["contents"][0];
    assert_eq!(contents["mimeType"], "application/json");
    let artifacts: Vec<serde_json::Value> =
        serde_json::from_str(contents["text"].as_str().unwrap()).unwrap();
    assert!(artifacts.is_empty());
}

#[tokio::test]
async fn register_then_read_artifact_by_id() {
    let server = setup();

    // Register
    let resp = server
        .handle_request(make_request(
            "tools/call",
            serde_json::json!({
                "name": "register_artifact",
                "arguments": {
                    "name": "readable",
                    "namespace": "org/test",
                    "tier": "project",
                    "format": "claude-md",
                    "owner": "owner",
                    "content": "# Readable\n\nTest content for reading.",
                    "author": "author",
                    "message": "msg"
                }
            }),
        ))
        .await;
    let artifact: serde_json::Value =
        serde_json::from_str(resp.result.unwrap()["content"][0]["text"].as_str().unwrap()).unwrap();
    let artifact_id = artifact["id"].as_str().unwrap();

    // Read by ID
    let resp = server
        .handle_request(make_request(
            "resources/read",
            serde_json::json!({"uri": format!("contextops://artifacts/{artifact_id}")}),
        ))
        .await;
    assert!(resp.error.is_none());

    let detail: serde_json::Value = serde_json::from_str(
        resp.result.unwrap()["contents"][0]["text"].as_str().unwrap(),
    )
    .unwrap();
    assert_eq!(detail["name"], "readable");

    // Read content
    let resp = server
        .handle_request(make_request(
            "resources/read",
            serde_json::json!({"uri": format!("contextops://artifacts/{artifact_id}/content")}),
        ))
        .await;
    assert!(resp.error.is_none());

    let content = resp.result.unwrap()["contents"][0]["text"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(content.contains("# Readable"));
}

#[tokio::test]
async fn read_pipeline_runs_empty() {
    let server = setup();
    let req = make_request(
        "resources/read",
        serde_json::json!({"uri": "contextops://pipeline/runs"}),
    );
    let resp = server.handle_request(req).await;
    assert!(resp.error.is_none());
}

#[tokio::test]
async fn read_unknown_resource_returns_error() {
    let server = setup();
    let req = make_request(
        "resources/read",
        serde_json::json!({"uri": "contextops://nonexistent"}),
    );
    let resp = server.handle_request(req).await;
    assert!(resp.error.is_some());
}

#[tokio::test]
async fn read_resolve_namespace() {
    let server = setup();
    let req = make_request(
        "resources/read",
        serde_json::json!({"uri": "contextops://resolve/org/test"}),
    );
    let resp = server.handle_request(req).await;
    assert!(resp.error.is_none());
}
