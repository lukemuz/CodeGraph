# CodeGraph Index Freshness Architecture

## Problem Statement
The CodeGraph index becomes stale when source files change. We need an architecture that keeps the index fresh without impacting performance or user experience.

## Proposed Solutions

### 1. Hybrid Approach (RECOMMENDED) ⭐

The best architecture combines multiple strategies based on context:

```rust
// src/freshness.rs
pub struct FreshnessManager {
    // File watching for active development
    watcher: Option<FileWatcher>,
    // Track file hashes to detect real changes
    file_hashes: HashMap<PathBuf, String>,
    // Incremental update queue
    update_queue: Arc<Mutex<Vec<PathBuf>>>,
    // Last full index time
    last_index_time: SystemTime,
}

impl FreshnessManager {
    pub async fn ensure_fresh(&mut self, graph: &mut CodeGraph) -> Result<()> {
        match self.get_freshness_strategy() {
            Strategy::Watch => self.start_watching(),
            Strategy::LazyCheck => self.check_and_update_if_stale(),
            Strategy::ForceRebuild => self.rebuild_full_index(),
        }
    }
}
```

**Implementation Plan:**

```toml
# Add to Cargo.toml
[dependencies]
notify = "6.1"        # File system watching
sha2 = "0.10"        # File content hashing
dashmap = "6.0"      # Concurrent hashmap for updates
```

**How it works:**
1. **On startup**: Quick staleness check (compare timestamps)
2. **During development**: File watcher with debouncing
3. **On MCP calls**: Validate specific files being queried
4. **Periodic**: Background refresh every 5 minutes

### 2. File Watching with Debouncing

```rust
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use notify_debouncer_mini::{new_debouncer, DebouncedEvent};

pub struct FileWatcher {
    debouncer: Debouncer<RecommendedWatcher>,
    pending_updates: Arc<Mutex<HashSet<PathBuf>>>,
}

impl FileWatcher {
    pub fn new(graph: Arc<Mutex<CodeGraph>>) -> Result<Self> {
        let (tx, rx) = std::sync::mpsc::channel();
        
        // Debounce for 500ms to avoid rapid rebuilds
        let mut debouncer = new_debouncer(Duration::from_millis(500), tx)?;
        
        debouncer.watcher().watch(
            Path::new("."),
            RecursiveMode::Recursive
        )?;
        
        // Process events in background
        tokio::spawn(async move {
            while let Ok(events) = rx.recv() {
                for event in events {
                    if is_source_file(&event.path) {
                        // Queue for incremental update
                        pending_updates.lock().insert(event.path);
                    }
                }
                // Batch process updates
                Self::process_updates(&graph, &pending_updates).await;
            }
        });
        
        Ok(Self { debouncer, pending_updates })
    }
}
```

**Pros:**
- Real-time updates during development
- Minimal latency for LLM queries
- Efficient with debouncing

**Cons:**
- Uses resources continuously
- Complex to implement correctly
- May miss changes if watcher fails

### 3. Lazy Validation with Smart Caching

```rust
impl McpServer {
    async fn validate_freshness(&mut self, scope: Option<&Path>) -> Result<()> {
        // Only check files in the query scope
        let files_to_check = match scope {
            Some(path) => self.graph.get_files_in_path(path),
            None => self.graph.get_all_files(),
        };
        
        for file in files_to_check {
            let current_mtime = fs::metadata(&file)?.modified()?;
            let indexed_mtime = self.graph.get_file_mtime(&file);
            
            if current_mtime > indexed_mtime {
                // Incremental update just this file
                self.update_single_file(&file).await?;
            }
        }
        Ok(())
    }
    
    // Called before each tool execution
    async fn handle_tool_call(&mut self, tool: &str, params: &Value) -> Result<Value> {
        // Extract scope from params if available
        let scope = extract_scope(params);
        self.validate_freshness(scope).await?;
        
        // Now execute with fresh index
        self.execute_tool(tool, params).await
    }
}
```

**Pros:**
- No background resource usage
- Simple to implement
- Always accurate when queried

**Cons:**
- Adds latency to first query
- May do redundant checks

### 4. Incremental Updates Only

Modify the graph structure to support efficient incremental updates:

```rust
impl CodeGraph {
    // Track which nodes belong to which files
    pub file_nodes: HashMap<PathBuf, Vec<NodeIndex>>,
    
    pub fn update_file(&mut self, file: &Path, parser: &Parser) -> Result<()> {
        // 1. Remove old nodes from this file
        if let Some(old_nodes) = self.file_nodes.get(file) {
            for &node in old_nodes.iter().rev() {
                // Remove node and all its edges
                self.graph.remove_node(node);
            }
        }
        
        // 2. Parse the file again
        let content = fs::read_to_string(file)?;
        let new_nodes = parser.parse_file(file, &content)?;
        
        // 3. Add new nodes
        let mut node_indices = Vec::new();
        for function in new_nodes {
            let idx = self.add_function(function);
            node_indices.push(idx);
        }
        
        // 4. Re-establish edges (this is the complex part)
        self.rebuild_edges_for_file(file, &node_indices)?;
        
        // 5. Update the file mapping
        self.file_nodes.insert(file.to_path_buf(), node_indices);
        
        Ok(())
    }
}
```

**Challenge:** Re-establishing edges after incremental updates requires tracking call sites.

### 5. Git Integration

```bash
#!/bin/bash
# .git/hooks/post-checkout
codegraph index . --incremental

# .git/hooks/post-merge  
codegraph index . --incremental

# .git/hooks/post-commit
FILES=$(git diff --name-only HEAD~1)
codegraph update --files $FILES
```

**Pros:**
- Integrates with existing workflow
- No runtime overhead
- Works with CI/CD

**Cons:**
- Misses uncommitted changes
- Requires git hooks setup
- Not real-time

## Recommended Implementation Strategy

### Phase 1: Timestamp-based Lazy Validation (Quick Win)
```rust
// Add to existing McpServer
impl McpServer {
    fn is_index_stale(&self) -> Result<bool> {
        let index_time = fs::metadata(".codegraph/index.bin")?.modified()?;
        
        // Check a sample of files for changes
        for file in self.graph.file_index.keys().take(10) {
            if let Ok(meta) = fs::metadata(file) {
                if meta.modified()? > index_time {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
    
    async fn ensure_fresh(&mut self) -> Result<()> {
        if self.is_index_stale()? {
            eprintln!("Index is stale, rebuilding...");
            self.rebuild_index().await?;
        }
        Ok(())
    }
}
```

### Phase 2: Add File Watching (Development Mode)
```bash
# Start with auto-refresh
codegraph serve --watch

# Or traditional mode
codegraph serve
```

### Phase 3: Incremental Updates
- Track file → nodes mapping
- Implement single-file updates
- Smart edge reconstruction

## Performance Considerations

| Strategy | Initial Load | Query Latency | Memory Usage | CPU Usage |
|----------|-------------|---------------|--------------|-----------|
| File Watching | Fast | Fast | Medium | Medium (continuous) |
| Lazy Check | Slow (first) | Variable | Low | Low (on-demand) |
| Git Hooks | Fast | Fast | Low | None (runtime) |
| Hybrid | Fast | Fast | Medium | Low-Medium |

## Configuration Options

```toml
# .codegraph/config.toml
[freshness]
strategy = "hybrid"  # watch | lazy | git | manual

[watch]
enabled = true
debounce_ms = 500
ignore_patterns = ["*.pyc", "__pycache__", ".git"]

[lazy]
check_on_startup = true
check_interval_seconds = 300  # Check every 5 minutes
sample_size = 10  # Files to check for staleness

[incremental]
enabled = true
batch_size = 5  # Update files in batches
```

## CLI Commands

```bash
# Force rebuild
codegraph index . --force

# Incremental update specific files
codegraph update src/main.py src/utils.py

# Watch mode
codegraph serve --watch

# Check freshness without updating
codegraph check --verbose

# Show index statistics
codegraph stats
```

## Decision Matrix

Choose based on your use case:

1. **Active Development** → File Watching
   - You're actively editing code
   - Want immediate updates
   - Don't mind resource usage

2. **CI/CD Integration** → Git Hooks
   - Index updates with commits
   - Clean, predictable updates
   - No runtime overhead

3. **Occasional Use** → Lazy Validation
   - Use CodeGraph sporadically
   - Want minimal resource usage
   - Can tolerate initial delay

4. **Large Codebases** → Incremental Updates
   - Thousands of files
   - Frequent small changes
   - Need optimal performance

5. **Best of All** → Hybrid Approach
   - Adapts to context
   - Optimizes for each scenario
   - Most complex to implement

## Next Steps

1. Start with lazy validation (easy, immediate benefit)
2. Add optional file watching for development
3. Implement incremental updates for efficiency
4. Consider git hooks for CI/CD integration

The hybrid approach gives the best user experience but requires more implementation effort. Start simple and evolve based on actual usage patterns.