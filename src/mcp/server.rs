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
                title: Some("Function Navigator".to_string()),
                description: "Explore a specific function and its code relationships. Use this when you want to understand how a function connects to the rest of the codebase - what it calls, what calls it, and related functions in the same file. Perfect for understanding data flow, tracing execution paths, or getting oriented in unfamiliar code. Example use cases: 'How does process_data work?', 'What functions does authenticate call?', 'Show me the call chain from main to database operations.'".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "function": {
                            "type": "string",
                            "description": "Exact name of the function to navigate to. Use the precise function name as it appears in the code (case-sensitive). Examples: 'process_data', 'UserService.createUser', 'calculateTotal'"
                        },
                        "depth": {
                            "type": "number",
                            "description": "How many levels deep to explore relationships. 1 (default) shows direct relationships only. Higher values show transitive relationships but may return large results. Use 1-2 for focused exploration, 3-5 for comprehensive analysis.",
                            "minimum": 1,
                            "maximum": 5,
                            "default": 1
                        }
                    },
                    "required": ["function"]
                }),
                output_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "function": {
                            "type": "object",
                            "description": "Details about the target function"
                        },
                        "calls": {
                            "type": "array", 
                            "description": "Functions called by this function"
                        },
                        "called_by": {
                            "type": "array",
                            "description": "Functions that call this function"
                        },
                        "siblings": {
                            "type": "array", 
                            "description": "Other functions in the same file"
                        },
                        "summary": {
                            "type": "string",
                            "description": "Human-readable summary of relationships"
                        }
                    }
                })),
                annotations: Some(crate::mcp::ToolAnnotations {
                    audience: Some(vec!["developer".to_string()]),
                    priority: Some(0.8),
                }),
            },
            ToolDefinition {
                name: "find".to_string(),
                title: Some("Function Finder".to_string()),
                description: "Search for functions across the codebase using fuzzy matching. Use this when you don't know the exact function name or want to discover functions related to a concept. Ideal for exploring unfamiliar codebases, finding functions by partial names, or discovering related functionality. The search combines exact matches, fuzzy matching, and regex patterns to find the most relevant functions. Example use cases: 'Find functions related to authentication', 'Search for data validation functions', 'What functions contain 'user' in their name?'".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search term for function names. Can be partial names, concepts, or patterns. Examples: 'auth' (finds authenticate, authorization), 'process' (finds processData, process_user), 'valid' (finds validate, is_valid). The search is case-insensitive and uses fuzzy matching."
                        },
                        "scope": {
                            "type": "string",
                            "description": "Optional path to limit search to specific files or directories. Use file paths (like 'src/auth.py') or directory paths (like 'src/') to narrow results. Leave empty to search the entire codebase. Examples: 'src/models/', 'utils.py', 'tests/'"
                        }
                    },
                    "required": ["query"]
                }),
                output_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "matches": {
                            "type": "array",
                            "description": "Functions matching the search query with confidence scores"
                        },
                        "grouped_by_file": {
                            "type": "object",
                            "description": "Results organized by file path"
                        },
                        "summary": {
                            "type": "string",
                            "description": "Human-readable summary of search results"
                        }
                    }
                })),
                annotations: Some(crate::mcp::ToolAnnotations {
                    audience: Some(vec!["developer".to_string()]),
                    priority: Some(0.9),
                }),
            },
            ToolDefinition {
                name: "impact".to_string(),
                title: Some("Impact Analyzer".to_string()),
                description: "Analyze the blast radius of changing a function - understand what would break if you modify, rename, or delete it. Essential for safe refactoring, assessing technical debt, and understanding code dependencies. Shows both direct callers and transitive impact through the entire call chain, plus provides a risk assessment. Use before making changes to existing functions, when planning refactoring, or to understand the scope of technical debt. Example use cases: 'Is it safe to modify this validation function?', 'What would break if I change this API endpoint?', 'How many functions depend on this utility?'".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "function": {
                            "type": "string",
                            "description": "Exact name of the function to analyze for impact. Must be the precise function name as it appears in the code. Examples: 'validateUser', 'DatabaseConnection.connect', 'calculatePrice'"
                        },
                        "include_tests": {
                            "type": "boolean",
                            "description": "Whether to include test files in the impact analysis. Set to true when you want to understand test coverage and what tests might need updating. Set to false (default) for cleaner analysis focused on production code. Including tests helps with comprehensive refactoring planning.",
                            "default": false
                        }
                    },
                    "required": ["function"]
                }),
                output_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "direct_callers": {
                            "type": "array",
                            "description": "Functions that directly call this function"
                        },
                        "transitive_impact": {
                            "type": "array", 
                            "description": "Functions indirectly affected through the call chain"
                        },
                        "affected_files": {
                            "type": "array",
                            "description": "Files that would be impacted by changes"
                        },
                        "test_files": {
                            "type": "array",
                            "description": "Test files that reference this function"
                        },
                        "risk_level": {
                            "type": "string",
                            "enum": ["low", "medium", "high"],
                            "description": "Assessment of change risk"
                        },
                        "summary": {
                            "type": "string",
                            "description": "Human-readable impact summary"
                        }
                    }
                })),
                annotations: Some(crate::mcp::ToolAnnotations {
                    audience: Some(vec!["developer".to_string()]),
                    priority: Some(1.0),
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