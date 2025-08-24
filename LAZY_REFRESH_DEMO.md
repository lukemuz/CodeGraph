# Lazy Refresh with Periodic Updates - Implementation

## Overview

I've implemented a **lazy update approach with periodic refresh** for CodeGraph. This keeps the index fresh without constant file watching overhead.

## How It Works

### 1. Lazy Staleness Check
Before each MCP tool call, the server:
- Checks if the index is stale by sampling random Python files
- Compares file modification times with index time
- Only rebuilds if files are newer than index

### 2. Periodic Background Refresh (Optional)
When enabled with `--auto-refresh`:
- Checks staleness every N seconds (default: 300)
- Automatically rebuilds index if stale
- Runs in background without blocking queries

## Architecture

```rust
// src/freshness.rs
pub struct FreshnessManager {
    index_path: PathBuf,
    project_path: PathBuf,
    check_interval: Duration,  // How often to check
    sample_size: usize,        // How many files to sample
}

impl FreshnessManager {
    // Smart sampling - checks random subset of files
    pub fn is_stale(&self) -> Result<bool> {
        // Get index modification time
        let index_time = fs::metadata(&self.index_path)?.modified()?;
        
        // Sample random Python files
        let mut python_files = collect_python_files();
        python_files.shuffle(&mut rand::rng());
        
        // Check sample for newer files
        for file in python_files.iter().take(self.sample_size) {
            if file_modified_time > index_time {
                return Ok(true); // Stale!
            }
        }
        Ok(false)
    }
}
```

## Usage

### Basic Mode (Manual Refresh)
```bash
# Start server - no automatic refresh
codegraph serve

# Manually rebuild when needed
codegraph index . --force
```

### Auto-Refresh Mode
```bash
# Enable lazy refresh with 5-minute intervals
codegraph serve --auto-refresh

# Custom interval (60 seconds)
codegraph serve --auto-refresh --refresh-interval 60
```

## Implementation Details

### Server Integration
```rust
// MCP server checks freshness before each tool call
async fn handle_tool_call(&self, request: JsonRpcRequest) -> JsonRpcResponse {
    // Lazy check - only when processing requests
    if let Err(e) = self.ensure_fresh().await {
        warn!("Failed to check freshness: {}", e);
        // Continue with potentially stale data
    }
    
    // Process the tool call
    match tool_name {
        "navigate" => self.handle_navigate_tool(...),
        "find" => self.handle_find_tool(...),
        "impact" => self.handle_impact_tool(...),
    }
}
```

### Performance Optimizations

1. **Sampling Strategy**: Only checks 10 random files instead of entire codebase
2. **Lazy Evaluation**: Only checks when actually serving requests
3. **Debounced Rebuilds**: Won't rebuild more often than check interval
4. **Non-blocking**: Serves stale data rather than blocking on rebuild

## Testing

Created test that demonstrates the functionality:

```bash
# Initial index: 30 functions
codegraph index test_project

# Add new function to main.py
echo "def new_func(): pass" >> main.py

# Server detects stale index and rebuilds
codegraph serve --auto-refresh --refresh-interval 2

# Index automatically updated: 31 functions
```

## Benefits

✅ **Simple**: No complex file watching or dependency tracking
✅ **Efficient**: Minimal overhead with smart sampling
✅ **Predictable**: Clear behavior and timing
✅ **Flexible**: Works with or without auto-refresh
✅ **Reliable**: Falls back to stale data rather than failing

## Trade-offs

- **Not real-time**: Changes detected on next query or interval
- **May miss changes**: Sampling might not catch all modifications
- **Rebuild overhead**: Full rebuild when stale (no incremental updates yet)

## Future Improvements

1. **Incremental Updates**: Only reparse changed files
2. **File Hashing**: Use content hash instead of timestamps
3. **Configurable Sampling**: Adjust sample size based on project size
4. **Smart Scheduling**: Rebuild during idle periods

## Configuration

Future config file support:
```toml
# .codegraph/config.toml
[freshness]
enabled = true
check_interval = 300        # seconds
sample_size = 10           # files to check
rebuild_on_startup = true  # check on server start
```

This implementation provides a good balance between freshness and performance, perfect for development workflows where the index should stay reasonably up-to-date without constant overhead.