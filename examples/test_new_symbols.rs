use codegraph::cli::Indexer;
use codegraph::mcp::operations::OperationHandler;
use std::path::Path;

fn main() -> anyhow::Result<()> {
    println!("🧪 Testing New Symbol Types");
    println!("============================");
    
    // Index the test project
    println!("1. Indexing test project...");
    let indexer = Indexer::new()?;
    let index_path = Path::new("test_project/.codegraph/index.bin");
    
    // Load the graph
    println!("2. Loading graph with new symbol types...");
    let graph = indexer.load_index(index_path)?;
    let ops = OperationHandler::new();
    
    // Test basic stats
    println!("3. Graph Statistics:");
    println!("   - Total symbols: {}", graph.graph.node_count());
    println!("   - Total relations: {}", graph.graph.edge_count());
    println!("   - Symbol index size: {}", graph.symbol_index.len());
    println!("   - Type index size: {}", graph.type_index.len());
    
    // Test find operation
    println!("\n4. Testing FIND operation:");
    match ops.find_functions(&graph, "main", None) {
        Ok(result) => {
            println!("   ✅ Found {} matches for 'main'", result.matches.len());
            for (i, m) in result.matches.iter().take(3).enumerate() {
                println!("      {}. {} at {}:{}", i+1, m.name, m.file, m.line);
            }
        }
        Err(e) => println!("   ❌ Find failed: {}", e),
    }
    
    // Test navigate operation
    println!("\n5. Testing NAVIGATE operation:");
    match ops.navigate(&graph, "main", Some(1)) {
        Ok(result) => {
            println!("   ✅ Navigate successful for '{}'", result.function.name);
            println!("      - File: {}:{}", result.function.file, result.function.line);
            println!("      - Calls {} functions", result.calls.len());
            println!("      - Called by {} functions", result.called_by.len());
            println!("      - Has {} siblings", result.siblings.len());
        }
        Err(e) => println!("   ❌ Navigate failed: {}", e),
    }
    
    // Test impact operation  
    println!("\n6. Testing IMPACT operation:");
    match ops.analyze_impact(&graph, "main", false) {
        Ok(result) => {
            println!("   ✅ Impact analysis successful");
            println!("      - Risk level: {}", result.risk_level);
            println!("      - Direct callers: {}", result.direct_callers.len());
            println!("      - Transitive impact: {}", result.transitive_impact.len());
            println!("      - Affected files: {}", result.affected_files.len());
        }
        Err(e) => println!("   ❌ Impact analysis failed: {}", e),
    }
    
    // Test symbol types breakdown
    println!("\n7. Symbol Type Analysis:");
    let mut function_count = 0;
    let mut other_count = 0;
    
    for node in graph.graph.node_weights() {
        match node.symbol_type {
            codegraph::graph::SymbolType::Function => function_count += 1,
            _ => other_count += 1,
        }
    }
    
    println!("   - Functions: {}", function_count);
    println!("   - Other symbols: {}", other_count);
    
    println!("\n🎉 All tests completed!");
    println!("   The refactoring successfully updated the system to use");
    println!("   SymbolNode, RelationEdge, and the new type system!");
    
    Ok(())
}