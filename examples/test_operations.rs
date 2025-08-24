use codegraph::cli::Indexer;
use codegraph::mcp::operations::OperationHandler;
use std::path::Path;

fn main() -> anyhow::Result<()> {
    println!("🔍 CODEGRAPH LLM CONTEXT EXAMPLES");
    println!("{}", "=".repeat(50));

    // Load our test project index
    let indexer = Indexer::new()?;
    let graph = indexer.load_index(Path::new("test_project/.codegraph/index.bin"))?;
    let handler = OperationHandler::new();

    println!("\n📊 GRAPH STATS:");
    println!("Functions indexed: {}", graph.graph.node_count());
    println!("Relationships: {}", graph.graph.edge_count());

    // Test 1: Navigate operation - "What does process_data function do?"
    println!("\n{}", "=".repeat(60));
    println!("🧭 QUERY 1: Navigate 'process_data' function");
    println!("{}", "=".repeat(60));
    
    match handler.navigate(&graph, "process_data", Some(2)) {
        Ok(result) => {
            println!("📋 LLM CONTEXT RETURNED:");
            println!("{}", serde_json::to_string_pretty(&result)?);
            
            println!("\n🤖 WHAT LLM SEES:");
            println!("Summary: {}", result.summary);
            println!("Function: {} at {}:{}", result.function.name, result.function.file, result.function.line);
            println!("Signature: {}", result.function.signature);
            println!("Calls {} functions, called by {} functions", 
                     result.calls.len(), result.called_by.len());
        }
        Err(e) => println!("❌ Error: {}", e),
    }

    // Test 2: Find operation - "Find all functions related to 'data'"
    println!("\n{}", "=".repeat(60));
    println!("🔍 QUERY 2: Find functions containing 'data'");
    println!("{}", "=".repeat(60));
    
    match handler.find_functions(&graph, "data", None) {
        Ok(result) => {
            println!("📋 LLM CONTEXT RETURNED:");
            println!("{}", serde_json::to_string_pretty(&result)?);
            
            println!("\n🤖 WHAT LLM SEES:");
            println!("Summary: {}", result.summary);
            println!("Found {} matches:", result.matches.len());
            for (i, func) in result.matches.iter().take(5).enumerate() {
                println!("  {}. {} ({}:{}) - confidence: {:.2}", 
                         i+1, func.name, func.file, func.line, func.confidence);
            }
        }
        Err(e) => println!("❌ Error: {}", e),
    }

    // Test 3: Impact analysis - "What would break if I change clean_data?"
    println!("\n{}", "=".repeat(60));
    println!("💥 QUERY 3: Impact analysis for 'clean_data'");
    println!("{}", "=".repeat(60));
    
    match handler.analyze_impact(&graph, "clean_data", false) {
        Ok(result) => {
            println!("📋 LLM CONTEXT RETURNED:");
            println!("{}", serde_json::to_string_pretty(&result)?);
            
            println!("\n🤖 WHAT LLM SEES:");
            println!("Summary: {}", result.summary);
            println!("Risk Level: {}", result.risk_level);
            println!("Direct callers: {}", result.direct_callers.len());
            println!("Affected files: {:?}", result.affected_files);
        }
        Err(e) => println!("❌ Error: {}", e),
    }

    Ok(())
}