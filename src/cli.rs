use crate::graph::CodeGraph;
use crate::parser::ParserManager;
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, warn};
use walkdir::WalkDir;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Index {
        #[arg(help = "Path to the project directory to index")]
        path: PathBuf,
        
        #[arg(short, long, help = "Output path for the index file")]
        output: Option<PathBuf>,
        
        #[arg(short, long, help = "Force rebuild even if index exists")]
        force: bool,
        
        #[arg(short, long, help = "Show verbose output")]
        verbose: bool,
    },
    
    Serve {
        #[arg(short, long, help = "Path to the index file")]
        index: Option<PathBuf>,
        
        #[arg(long, help = "Enable automatic freshness checking")]
        auto_refresh: bool,
        
        #[arg(long, help = "Freshness check interval in seconds (default: 300)")]
        refresh_interval: Option<u64>,
    },
    
    /// Run as MCP server (auto-indexes and auto-refreshes)
    Mcp {
        #[arg(short, long, help = "Project directory (defaults to current)")]
        project: Option<PathBuf>,
        
        #[arg(long, help = "Freshness check interval in seconds (default: 300)")]
        refresh_interval: Option<u64>,
    },
}

pub struct Indexer {
    parser_manager: ParserManager,
}

impl Indexer {
    pub fn new() -> Result<Self> {
        Ok(Self {
            parser_manager: ParserManager::new()?,
        })
    }

    pub fn index_project(&self, project_path: &Path, output_path: &Path, verbose: bool) -> Result<()> {
        info!("Starting to index project at: {}", project_path.display());
        
        let mut graph = CodeGraph::new();
        let mut file_count = 0;
        let mut function_count = 0;

        let supported_extensions = ["py", "js", "jsx", "mjs", "ts", "tsx", "rs"];
        
        for entry in WalkDir::new(project_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| {
                if let Some(ext) = e.path().extension() {
                    if let Some(ext_str) = ext.to_str() {
                        return supported_extensions.contains(&ext_str);
                    }
                }
                false
            })
        {
            let file_path = entry.path();
            
            if self.should_skip_file(file_path) {
                continue;
            }

            match fs::read_to_string(file_path) {
                Ok(content) => {
                    if verbose {
                        info!("Parsing: {}", file_path.display());
                    }
                    
                    let initial_node_count = graph.graph.node_count();
                    
                    if let Err(e) = self.parser_manager.parse_file(file_path, &content, &mut graph) {
                        warn!("Failed to parse {}: {}", file_path.display(), e);
                        continue;
                    }
                    
                    let new_functions = graph.graph.node_count() - initial_node_count;
                    function_count += new_functions;
                    file_count += 1;
                    
                    if verbose && new_functions > 0 {
                        info!("  Found {} functions", new_functions);
                    }
                }
                Err(e) => {
                    warn!("Failed to read {}: {}", file_path.display(), e);
                }
            }
        }

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let serialized = graph.serialize()?;
        fs::write(output_path, serialized)?;

        info!(
            "Indexing complete! Processed {} files, found {} functions",
            file_count, function_count
        );
        info!("Index saved to: {}", output_path.display());

        Ok(())
    }

    pub fn load_index(&self, index_path: &Path) -> Result<CodeGraph> {
        info!("Loading index from: {}", index_path.display());
        let data = fs::read(index_path)?;
        let graph = CodeGraph::deserialize(&data)?;
        info!("Index loaded with {} functions", graph.graph.node_count());
        Ok(graph)
    }

    fn should_skip_file(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        
        let skip_patterns = [
            "node_modules/",
            ".git/",
            "__pycache__/",
            ".pytest_cache/",
            "venv/",
            ".venv/",
            "env/",
            ".env/",
            "dist/",
            "build/",
            ".next/",
            ".nuxt/",
            "coverage/",
            ".nyc_output/",
            "target/",
            ".DS_Store",
        ];

        for pattern in &skip_patterns {
            if path_str.contains(pattern) {
                return true;
            }
        }

        if let Some(file_name) = path.file_name() {
            if let Some(name_str) = file_name.to_str() {
                if name_str.starts_with('.') && name_str != ".env" {
                    return true;
                }
            }
        }

        false
    }

    pub fn get_default_index_path(project_path: &Path) -> PathBuf {
        project_path.join(".codegraph").join("index.bin")
    }

    pub fn index_exists(&self, index_path: &Path) -> bool {
        index_path.exists() && index_path.is_file()
    }
}

pub async fn run_cli() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Index { path, output, force, verbose } => {
            let indexer = Indexer::new()?;
            
            let output_path = output.as_ref()
                .cloned()
                .unwrap_or_else(|| Indexer::get_default_index_path(path));

            if !force && indexer.index_exists(&output_path) {
                println!("Index already exists at {}. Use --force to rebuild.", output_path.display());
                return Ok(());
            }

            indexer.index_project(path, &output_path, *verbose)?;
        }
        
        Commands::Serve { index, auto_refresh, refresh_interval } => {
            let project_path = PathBuf::from(".");
            let index_path = index.as_ref()
                .cloned()
                .unwrap_or_else(|| Indexer::get_default_index_path(&project_path));

            let indexer = Indexer::new()?;
            
            // Auto-create index if it doesn't exist
            let graph = if !index_path.exists() {
                info!("Index not found, creating index for current directory...");
                eprintln!("📊 First run detected - indexing current directory...");
                indexer.index_project(&project_path, &index_path, false)?;
                eprintln!("✅ Indexing complete!");
                indexer.load_index(&index_path)?
            } else {
                indexer.load_index(&index_path)?
            };
            
            let mut server = crate::mcp::server::McpServer::new(graph);
            
            if *auto_refresh {
                let project_path = PathBuf::from(".");
                server = server.with_freshness(
                    index_path.clone(), 
                    project_path,
                    *refresh_interval
                );
                info!("Auto-refresh enabled with interval: {} seconds", 
                      refresh_interval.unwrap_or(300));
            }
            
            server.run_stdio().await?;
        }
        
        Commands::Mcp { project, refresh_interval } => {
            // Use project directory from env var if set (for MCP client config)
            let project_path = if let Ok(env_path) = std::env::var("CODEGRAPH_PROJECT") {
                PathBuf::from(env_path)
            } else {
                project.as_ref().cloned().unwrap_or_else(|| PathBuf::from("."))
            };
            
            let index_path = Indexer::get_default_index_path(&project_path);
            let indexer = Indexer::new()?;
            
            // Auto-create or update index
            let graph = if !index_path.exists() {
                info!("Creating index for project: {}", project_path.display());
                eprintln!("📊 Indexing project at {}...", project_path.display());
                indexer.index_project(&project_path, &index_path, false)?;
                eprintln!("✅ Indexing complete!");
                indexer.load_index(&index_path)?
            } else {
                indexer.load_index(&index_path)?
            };
            
            // Always enable auto-refresh in MCP mode
            let server = crate::mcp::server::McpServer::new(graph)
                .with_freshness(
                    index_path.clone(),
                    project_path,
                    *refresh_interval
                );
            
            info!("MCP server starting with auto-refresh (interval: {} seconds)", 
                  refresh_interval.unwrap_or(300));
            
            server.run_stdio().await?;
        }
    }

    Ok(())
}