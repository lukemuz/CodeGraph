mod cli;
mod graph;
mod parser;
mod resolver;
mod mcp;

use anyhow::Result;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Configure tracing to use stderr to avoid interfering with MCP protocol on stdout
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();
    cli::run_cli().await
}
