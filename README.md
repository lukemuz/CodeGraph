# CodeGraph 🕸️

A graph-based code navigation tool for LLMs that provides intelligent code context through the Model Context Protocol (MCP). CodeGraph builds comprehensive function call graphs from your codebase and offers three powerful operations to help LLMs understand code relationships and dependencies.

## 🚀 What is CodeGraph?

CodeGraph solves a fundamental problem: **LLMs need better code context than traditional grep/glob tools can provide**. Instead of just finding text matches, CodeGraph understands your code's structure and relationships.

### Key Features

- 🌐 **Function Call Graphs**: Build directed graphs showing how functions call each other
- 🧭 **Navigate**: Explore function relationships (what it calls, what calls it, siblings)
- 🔍 **Find**: Intelligent function search with fuzzy matching and confidence scores
- 💥 **Impact Analysis**: Understand what would break if you change a function
- 🔌 **MCP Integration**: Works seamlessly with Claude Desktop, VS Code, and other MCP clients
- ⚡ **Tree-sitter Parsing**: Accurate AST-based analysis (supports Python, JavaScript, TypeScript, and Rust)

## 🛠️ Installation

### Option 1: Quick Install (Recommended)

Install CodeGraph with our one-liner script:

```bash
curl -fsSL https://raw.githubusercontent.com/lukemuz/CodeGraph/main/install.sh | bash
```

This will:
- Detect your OS and architecture automatically
- Download the latest pre-built binary
- Install it to `~/.local/bin/codegraph`
- Show you how to configure it with Claude Code or Cursor

### Option 2: Manual Download

Download pre-built binaries from [GitHub Releases](https://github.com/lukemuz/CodeGraph/releases):

- **Linux (x64)**: `codegraph-x86_64-unknown-linux-musl.tar.gz`
- **macOS (Intel)**: `codegraph-x86_64-apple-darwin.tar.gz` 
- **macOS (Apple Silicon)**: `codegraph-aarch64-apple-darwin.tar.gz`
- **Windows**: `codegraph-x86_64-pc-windows-msvc.zip`

### Option 3: Install from Source

If you have Rust installed:

```bash
cargo install --git https://github.com/lukemuz/CodeGraph
```

### Option 4: Build from Source

```bash
git clone https://github.com/lukemuz/CodeGraph.git
cd CodeGraph
cargo build --release
# Binary at ./target/release/codegraph
```

## 📖 Quick Start

### Zero Configuration Setup 🎉

CodeGraph now works with **zero configuration**! Just install and point it at your project - it will automatically:
- Create an index on first use
- Keep the index updated with auto-refresh
- Work seamlessly with your MCP client

### Manual Indexing (Optional)

For large projects, you may want to pre-build the index:

```bash
# Index the current directory  
codegraph index .

# Or index a specific project
codegraph index /path/to/your/project

# Force rebuild if index exists
codegraph index . --force --verbose
```

This creates a `.codegraph/index.bin` file containing the function graph.

### 2. Connect with MCP Clients

#### Claude Code Desktop (Recommended)

Add CodeGraph with a single command:

```bash
claude mcp add codegraph -- codegraph mcp
```

For a specific project directory:

```bash
claude mcp add codegraph --env CODEGRAPH_PROJECT=/path/to/project -- codegraph mcp
```

#### Cursor

Add to `~/.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "codegraph": {
      "command": "codegraph",
      "args": ["mcp"],
      "env": {
        "CODEGRAPH_PROJECT": "/path/to/your/project"
      }
    }
  }
}
```

#### Other MCP Clients

For any MCP-compatible client, use:

```json
{
  "mcpServers": {
    "codegraph": {
      "command": "/path/to/codegraph",
      "args": ["mcp"],
      "env": {
        "CODEGRAPH_PROJECT": "/path/to/your/project"
      }
    }
  }
}
```

### 3. Start Using CodeGraph

That's it! CodeGraph will automatically:
- Index your project on first use (no manual setup needed!)
- Keep the index fresh with auto-refresh
- Use the `CODEGRAPH_PROJECT` environment variable or current directory
- Provide the three core operations: Navigate, Find, and Impact Analysis

## 🎯 Core Operations

CodeGraph provides three powerful operations for code navigation:

### 🧭 Navigate (Function Navigator)

Explore a specific function and its code relationships. Perfect for understanding data flow, tracing execution paths, or getting oriented in unfamiliar code:

**Input:**
```json
{
  "function": "process_data",
  "depth": 2
}
```

**Output:**
```json
{
  "function": {
    "name": "process_data",
    "file": "main.py",
    "line": 15,
    "signature": "def process_data(data_list)",
    "language": "Python"
  },
  "calls": [
    {"name": "clean_data", "file": "main.py", "line": 21, "confidence": 1.0},
    {"name": "validate_data", "file": "main.py", "line": 25, "confidence": 1.0},
    {"name": "format_output", "file": "main.py", "line": 33, "confidence": 1.0}
  ],
  "called_by": [
    {"name": "_do_processing", "file": "main.py", "line": 72, "confidence": 1.0}
  ],
  "siblings": [
    {"name": "hello_world", "file": "main.py", "line": 6, "confidence": 1.0}
  ],
  "summary": "Function 'process_data' calls 3 functions, is called by 1 function, and has 20 siblings."
}
```

**Use Cases:**
- 🔄 Understanding data flow through function calls
- 📁 Seeing all functions in the same file (siblings)  
- 🕳️ Finding entry points and leaf functions
- 🧭 Getting oriented in unfamiliar codebases
- 📊 Tracing execution paths and call chains

### 🔍 Find (Function Finder)

Search for functions across the codebase using fuzzy matching. Ideal when you don't know exact function names or want to discover functions related to a concept:

**Input:**
```json
{
  "query": "data",
  "scope": "src/"
}
```

**Output:**
```json
{
  "matches": [
    {"name": "process_data", "confidence": 0.82, "file": "main.py", "line": 15},
    {"name": "clean_data", "confidence": 0.82, "file": "main.py", "line": 21},
    {"name": "validate_data", "confidence": 0.82, "file": "utils.py", "line": 45}
  ],
  "grouped_by_file": {
    "main.py": [...],
    "utils.py": [...]
  },
  "summary": "Found 3 functions matching 'data' in src/."
}
```

**Search Strategy:**
1. **Exact matches** from tree-sitter parsing (confidence: 1.0)
2. **Fuzzy matching** with SkimMatcherV2 (confidence: 0.3-1.0)  
3. **Regex fallback** for broader patterns (confidence: 0.3-0.9)

**Use Cases:**
- 🎯 Finding functions by partial names or concepts
- 📂 Scoped searches within specific directories
- 🔗 Discovering related functionality across the codebase
- 🔍 Exploring unfamiliar codebases to understand structure
- 💡 Finding functions when you only remember part of the name

### 💥 Impact Analysis (Impact Analyzer)

Analyze the blast radius of changing a function - understand what would break if you modify, rename, or delete it. Essential for safe refactoring and assessing technical debt:

**Input:**
```json
{
  "function": "clean_data",
  "include_tests": false
}
```

**Output:**
```json
{
  "direct_callers": [
    {"name": "process_data", "file": "main.py", "line": 15}
  ],
  "transitive_impact": [
    {"name": "_do_processing", "file": "main.py", "line": 72},
    {"name": "main", "file": "main.py", "line": 78}
  ],
  "affected_files": ["main.py", "utils.py"],
  "test_files": [],
  "risk_level": "medium",
  "summary": "Changing 'clean_data' would directly affect 1 function and transitively impact 2 functions. Risk level: medium."
}
```

**Risk Assessment:**
- **Low**: 0-2 total affected functions
- **Medium**: 3-10 total affected functions  
- **High**: 10+ total affected functions

**Use Cases:**
- ⚠️ Assessing refactoring safety before making changes
- 🧪 Understanding test coverage and what tests need updating
- 🔄 Planning breaking changes and their scope
- 💸 Evaluating technical debt impact
- 🔍 Understanding function dependency chains

## 🎨 Example Workflow

Here's how CodeGraph helps with a real coding task:

### Scenario: Adding validation to a data processing pipeline

1. **🔍 Find the entry point:**
   ```
   Query: "process"
   → Found: process_data, process_user_input, batch_process
   ```

2. **🧭 Navigate to understand the flow:**
   ```
   Navigate: "process_data"
   → Calls: clean_data → validate_data → format_output
   → Called by: _do_processing, main
   ```

3. **💥 Check impact before changes:**
   ```
   Impact: "validate_data"  
   → Risk: Low (2 direct callers)
   → Safe to modify validation logic
   ```

4. **🔍 Find related validation functions:**
   ```
   Query: "valid"
   → Found: validate_data, is_valid, validate_user
   ```

The LLM now has complete context about the validation pipeline, its dependencies, and the safety of making changes!

## 🌟 What's New

### v0.1.0 - Zero Configuration Release
- **Auto-indexing**: No manual setup required - just install and go!
- **Auto-refresh**: Keeps your index updated as you code
- **Smart defaults**: Works out of the box with sensible settings
- **MCP command**: New `mcp` subcommand for simplified MCP server usage
- **Environment support**: Use `CODEGRAPH_PROJECT` to specify project paths

## 🏗️ How It Works

### Architecture

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Tree-sitter   │───▶│   Function Graph │───▶│   MCP Server    │
│     Parser      │    │   (petgraph)     │    │  (JSON-RPC 2.0) │
└─────────────────┘    └──────────────────┘    └─────────────────┘
        │                       │                       │
        ▼                       ▼                       ▼
   Parse Python           Build directed         Serve tools via
   AST accurately         call graph with        stdin/stdout to
                         confidence scores       MCP clients
```

### Technical Details

- **Parser**: Tree-sitter for accurate AST parsing (Python, JavaScript, TypeScript, Rust support)
- **Graph**: Petgraph for efficient directed graph operations
- **Storage**: Bincode for fast serialization/deserialization
- **Protocol**: Full MCP compliance with JSON-RPC 2.0
- **Search**: Multi-layered resolution with fuzzy matching
- **Performance**: Indexed lookups, cached parsing results

## 🔮 Future Enhancements

- 📝 **More Languages**: Go, C++, Java, C# support
- 🌐 **Cross-language**: Track calls between different languages
- 📚 **Documentation**: Extract and index function docstrings
- 🔄 **Live Updates**: Watch file changes and update index
- 📊 **Metrics**: Function complexity, usage statistics
- 🧪 **Test Integration**: Map functions to their tests

## 🤝 Contributing

CodeGraph is built with Rust and uses modern tools:

- **Tree-sitter**: For accurate code parsing
- **Petgraph**: For graph data structures
- **Tokio**: For async I/O and concurrency
- **Serde**: For JSON serialization
- **Clap**: For CLI interface

## 📄 License

MIT License - see LICENSE file for details.

---

*CodeGraph: Because LLMs deserve better than grep.* 🚀