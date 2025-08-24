use crate::graph::CodeGraph;
use crate::mcp::{
    JsonRpcRequest, JsonRpcResponse, JsonRpcError, ToolResult, ContentBlock,
    InitializeParams, InitializeResult, ServerCapabilities, ServerInfo, ToolsCapability,
    ToolDefinition, NavigateParams, ImpactParams, FindParams,
};
use crate::mcp::operations::OperationHandler;
use crate::freshness::FreshnessManager;
use anyhow::Result;
use serde_json::{json, Value};
use std::sync::Arc;
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{Mutex, RwLock};
use tracing::{info, warn, error};

pub struct McpServer {
    graph: Arc<RwLock<CodeGraph>>,
    operations: OperationHandler,
    initialized: Arc<Mutex<bool>>,
    freshness_manager: Option<Arc<Mutex<FreshnessManager>>>,
    index_path: PathBuf,
    project_path: PathBuf,
}

impl McpServer {
    pub fn new(graph: CodeGraph) -> Self {
        Self {
            graph: Arc::new(RwLock::new(graph)),
            operations: OperationHandler::new(),
            initialized: Arc::new(Mutex::new(false)),
            freshness_manager: None,
            index_path: PathBuf::from(".codegraph/index.bin"),
            project_path: PathBuf::from("."),
        }
    }
    
    pub fn with_freshness(mut self, index_path: PathBuf, project_path: PathBuf, check_interval: Option<u64>) -> Self {
        let mut manager = FreshnessManager::new(index_path.clone(), project_path.clone());
        
        if let Some(interval) = check_interval {
            manager = manager.with_interval(interval);
        }
        
        self.freshness_manager = Some(Arc::new(Mutex::new(manager)));
        self.index_path = index_path;
        self.project_path = project_path;
        self
    }
    
    async fn ensure_fresh(&self) -> Result<()> {
        if let Some(ref manager) = self.freshness_manager {
            let mgr = manager.lock().await;
            if mgr.is_stale()? {
                info!("Index is stale, rebuilding...");
                drop(mgr); // Release lock before rebuilding
                
                // Rebuild the index
                let indexer = crate::cli::Indexer::new()?;
                indexer.index_project(&self.project_path, &self.index_path, false)?;
                
                // Reload the graph
                let new_graph = indexer.load_index(&self.index_path)?;
                let mut graph = self.graph.write().await;
                *graph = new_graph;
                
                info!("Index rebuilt successfully");
            }
        }
        Ok(())
    }

    pub async fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        match request.method.as_str() {
            "initialize" => self.handle_initialize(request).await,
            "tools/list" => self.handle_tools_list(request).await,
            "tools/call" => self.handle_tool_call(request).await,
            _ => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: format!("Method not found: {}", request.method),
                    data: None,
                }),
                id: request.id,
            },
        }
    }

    async fn handle_initialize(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        match serde_json::from_value::<InitializeParams>(request.params) {
            Ok(params) => {
                info!("Initializing MCP server with client: {}", params.client_info.name);
                
                let mut initialized = self.initialized.lock().await;
                *initialized = true;
                
                let result = InitializeResult {
                    protocol_version: "2024-11-05".to_string(),
                    capabilities: ServerCapabilities {
                        tools: Some(ToolsCapability {
                            list_changed: None,
                        }),
                    },
                    server_info: ServerInfo {
                        name: "CodeGraph".to_string(),
                        version: "1.0.0".to_string(),
                    },
                };

                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: Some(serde_json::to_value(result).unwrap()),
                    error: None,
                    id: request.id,
                }
            }
            Err(e) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(JsonRpcError {
                    code: -32602,
                    message: format!("Invalid params: {}", e),
                    data: None,
                }),
                id: request.id,
            },
        }
    }

    async fn handle_tools_list(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let tools = vec![
            ToolDefinition {
                name: "navigate".to_string(),
                description: "Navigate to a function and see its relationships (calls, called by, siblings)".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "function": {
                            "type": "string",
                            "description": "Name of the function to navigate to"
                        },
                        "depth": {
                            "type": "number",
                            "description": "Depth of navigation (default: 1)",
                            "minimum": 1,
                            "maximum": 5
                        }
                    },
                    "required": ["function"]
                }),
            },
            ToolDefinition {
                name: "find".to_string(),
                description: "Find functions matching a query string".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search query for function names"
                        },
                        "scope": {
                            "type": "string",
                            "description": "Optional file path to limit search scope"
                        }
                    },
                    "required": ["query"]
                }),
            },
            ToolDefinition {
                name: "impact".to_string(),
                description: "Analyze the impact of changing a function (who calls it, transitive effects)".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "function": {
                            "type": "string",
                            "description": "Name of the function to analyze"
                        },
                        "include_tests": {
                            "type": "boolean",
                            "description": "Whether to include test files in impact analysis (default: false)"
                        }
                    },
                    "required": ["function"]
                }),
            },
        ];

        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(json!({ "tools": tools })),
            error: None,
            id: request.id,
        }
    }

    async fn handle_tool_call(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let initialized = *self.initialized.lock().await;
        if !initialized {
            return JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(JsonRpcError {
                    code: -32002,
                    message: "Server not initialized".to_string(),
                    data: None,
                }),
                id: request.id,
            };
        }
        
        // Check freshness before processing tool call
        if let Err(e) = self.ensure_fresh().await {
            warn!("Failed to check freshness: {}", e);
            // Continue anyway - better to serve stale data than fail
        }

        // Extract tool name and arguments from params
        let tool_name = request.params.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let arguments = request.params.get("arguments").unwrap_or(&Value::Null);

        match tool_name {
            "navigate" => self.handle_navigate_tool(request.id, arguments.clone()).await,
            "find" => self.handle_find_tool(request.id, arguments.clone()).await,
            "impact" => self.handle_impact_tool(request.id, arguments.clone()).await,
            _ => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(JsonRpcError {
                    code: -32602,
                    message: format!("Unknown tool: {}", tool_name),
                    data: None,
                }),
                id: request.id,
            },
        }
    }

    async fn handle_navigate_tool(&self, id: Value, arguments: Value) -> JsonRpcResponse {
        match serde_json::from_value::<NavigateParams>(arguments) {
            Ok(params) => {
                let graph = self.graph.read().await;
                match self.operations.navigate(&*graph, &params.function, params.depth) {
                    Ok(result) => {
                        let content = vec![ContentBlock {
                            content_type: "text".to_string(),
                            text: serde_json::to_string_pretty(&result).unwrap(),
                        }];

                        JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            result: Some(serde_json::to_value(ToolResult {
                                content,
                                is_error: None,
                            }).unwrap()),
                            error: None,
                            id,
                        }
                    }
                    Err(e) => {
                        let content = vec![ContentBlock {
                            content_type: "text".to_string(),
                            text: format!("Error: {}", e),
                        }];

                        JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            result: Some(serde_json::to_value(ToolResult {
                                content,
                                is_error: Some(true),
                            }).unwrap()),
                            error: None,
                            id,
                        }
                    }
                }
            }
            Err(e) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(JsonRpcError {
                    code: -32602,
                    message: format!("Invalid navigate parameters: {}", e),
                    data: None,
                }),
                id,
            },
        }
    }

    async fn handle_find_tool(&self, id: Value, arguments: Value) -> JsonRpcResponse {
        match serde_json::from_value::<FindParams>(arguments) {
            Ok(params) => {
                let scope = params.scope.as_ref().map(|s| std::path::Path::new(s));
                let graph = self.graph.read().await;
                match self.operations.find_functions(&*graph, &params.query, scope) {
                    Ok(result) => {
                        let content = vec![ContentBlock {
                            content_type: "text".to_string(),
                            text: serde_json::to_string_pretty(&result).unwrap(),
                        }];

                        JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            result: Some(serde_json::to_value(ToolResult {
                                content,
                                is_error: None,
                            }).unwrap()),
                            error: None,
                            id,
                        }
                    }
                    Err(e) => {
                        let content = vec![ContentBlock {
                            content_type: "text".to_string(),
                            text: format!("Error: {}", e),
                        }];

                        JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            result: Some(serde_json::to_value(ToolResult {
                                content,
                                is_error: Some(true),
                            }).unwrap()),
                            error: None,
                            id,
                        }
                    }
                }
            }
            Err(e) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(JsonRpcError {
                    code: -32602,
                    message: format!("Invalid find parameters: {}", e),
                    data: None,
                }),
                id,
            },
        }
    }

    async fn handle_impact_tool(&self, id: Value, arguments: Value) -> JsonRpcResponse {
        match serde_json::from_value::<ImpactParams>(arguments) {
            Ok(params) => {
                let graph = self.graph.read().await;
                match self.operations.analyze_impact(&*graph, &params.function, params.include_tests) {
                    Ok(result) => {
                        let content = vec![ContentBlock {
                            content_type: "text".to_string(),
                            text: serde_json::to_string_pretty(&result).unwrap(),
                        }];

                        JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            result: Some(serde_json::to_value(ToolResult {
                                content,
                                is_error: None,
                            }).unwrap()),
                            error: None,
                            id,
                        }
                    }
                    Err(e) => {
                        let content = vec![ContentBlock {
                            content_type: "text".to_string(),
                            text: format!("Error: {}", e),
                        }];

                        JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            result: Some(serde_json::to_value(ToolResult {
                                content,
                                is_error: Some(true),
                            }).unwrap()),
                            error: None,
                            id,
                        }
                    }
                }
            }
            Err(e) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(JsonRpcError {
                    code: -32602,
                    message: format!("Invalid impact parameters: {}", e),
                    data: None,
                }),
                id,
            },
        }
    }

    pub async fn run_stdio(&self) -> Result<()> {
        let stdin = tokio::io::stdin();
        let mut reader = BufReader::new(stdin);
        let mut stdout = tokio::io::stdout();
        let mut line = String::new();

        info!("MCP server starting on stdio");

        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    info!("EOF reached, shutting down");
                    break;
                }
                Ok(_) => {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }

                    match serde_json::from_str::<JsonRpcRequest>(line) {
                        Ok(request) => {
                            info!("Received request: {}", request.method);
                            let response = self.handle_request(request).await;
                            
                            match serde_json::to_string(&response) {
                                Ok(response_json) => {
                                    if let Err(e) = stdout.write_all(response_json.as_bytes()).await {
                                        error!("Failed to write response: {}", e);
                                        break;
                                    }
                                    if let Err(e) = stdout.write_all(b"\n").await {
                                        error!("Failed to write newline: {}", e);
                                        break;
                                    }
                                    if let Err(e) = stdout.flush().await {
                                        error!("Failed to flush response: {}", e);
                                        break;
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to serialize response: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to parse request '{}': {}", line, e);
                            let error_response = JsonRpcResponse {
                                jsonrpc: "2.0".to_string(),
                                result: None,
                                error: Some(JsonRpcError {
                                    code: -32700,
                                    message: format!("Parse error: {}", e),
                                    data: None,
                                }),
                                id: Value::Null,
                            };
                            
                            if let Ok(response_json) = serde_json::to_string(&error_response) {
                                let _ = stdout.write_all(response_json.as_bytes()).await;
                                let _ = stdout.write_all(b"\n").await;
                                let _ = stdout.flush().await;
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to read from stdin: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }
}