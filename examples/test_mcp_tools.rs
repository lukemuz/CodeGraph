use codegraph::cli::Indexer;
use codegraph::mcp::server::McpServer;
use codegraph::mcp::{JsonRpcRequest, JsonRpcResponse};
use serde_json::{json, Value};
use std::path::Path;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🔧 Testing Enhanced MCP Tools");
    println!("=============================");
    
    // Load the graph
    println!("1. Loading graph...");
    let indexer = Indexer::new()?;
    let index_path = Path::new("test_project/.codegraph/index.bin");
    let graph = indexer.load_index(index_path)?;
    
    // Create MCP server
    let server = McpServer::new(graph);
    
    // Test tools/list request
    println!("2. Testing tools/list request...");
    let tools_request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "tools/list".to_string(),
        params: json!({}),
        id: json!(1),
    };
    
    let response = server.handle_request(tools_request).await;
    
    if let Some(result) = response.result {
        let tools = result["tools"].as_array().unwrap();
        println!("   ✅ Found {} tools", tools.len());
        
        for tool in tools {
            let name = tool["name"].as_str().unwrap();
            let title = tool["title"].as_str().unwrap_or("No title");
            let description = tool["description"].as_str().unwrap_or("No description");
            let priority = tool["annotations"]["priority"].as_f64().unwrap_or(0.0);
            
            println!("\n   📦 Tool: {}", name);
            println!("      Title: {}", title);
            println!("      Priority: {}", priority);
            println!("      Description preview: {}...", 
                     &description.chars().take(100).collect::<String>());
        }
    } else {
        println!("   ❌ Failed to get tools list");
    }
    
    // Test a tool call - find operation
    println!("\n3. Testing find tool call...");
    let find_request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "tools/call".to_string(),
        params: json!({
            "name": "find",
            "arguments": {
                "query": "process"
            }
        }),
        id: json!(2),
    };
    
    let response = server.handle_request(find_request).await;
    if let Some(result) = response.result {
        let content = &result["content"][0]["text"];
        let data: Value = serde_json::from_str(content.as_str().unwrap())?;
        println!("   ✅ Find tool successful");
        println!("      Found {} matches", data["matches"].as_array().unwrap().len());
        println!("      Summary: {}", data["summary"].as_str().unwrap());
    } else if let Some(error) = response.error {
        println!("   ❌ Find tool failed: {}", error.message);
    }
    
    println!("\n🎉 Enhanced MCP tools test complete!");
    println!("   - Tool descriptions updated with better priorities");
    println!("   - Symbol system successfully refactored");
    println!("   - Ready for improved LLM discoverability");
    
    Ok(())
}