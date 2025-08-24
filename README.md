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
- ⚡ **Tree-sitter Parsing**: Accurate AST-based analysis (currently supports Python)

## 🛠️ Installation

```bash
# Clone the repository
git clone https://github.com/your-username/codegraph.git
cd codegraph

# Build the project
cargo build --release

# The binary will be available at ./target/release/codegraph
```

## 📖 Quick Start

### 1. Index Your Project

First, build an index of your codebase:

```bash
# Index the current directory
./target/release/codegraph index .

# Or index a specific project
./target/release/codegraph index /path/to/your/project

# Force rebuild if index exists
./target/release/codegraph index . --force --verbose
```

This creates a `.codegraph/index.bin` file containing the function graph.

### 2. Start the MCP Server

```bash
# Start the MCP server (uses stdin/stdout)
./target/release/codegraph serve

# Or specify a custom index path
./target/release/codegraph serve --index /path/to/index.bin
```

### 3. Connect with MCP Client

The server follows the Model Context Protocol and can be used with any MCP client. Here's an example configuration for Claude Desktop:

```json
{
  "mcpServers": {
    "codegraph": {
      "command": "/path/to/codegraph",
      "args": ["serve", "--index", "/path/to/your/project/.codegraph/index.bin"]
    }
  }
}
```

## 🎯 Core Operations

CodeGraph provides three powerful operations for code navigation:

### 🧭 Navigate

Explore a function and its relationships:

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

### 🔍 Find

Search for functions with intelligent matching:

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
- 🎯 Finding functions by partial names
- 📂 Scoped searches within directories  
- 🔗 Discovering related functionality

### 💥 Impact Analysis

Understand what would break if you change a function:

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
- ⚠️ Assessing refactoring safety
- 🧪 Understanding test coverage needs
- 🔄 Planning breaking changes

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

- **Parser**: Tree-sitter for accurate AST parsing (Python support)
- **Graph**: Petgraph for efficient directed graph operations
- **Storage**: Bincode for fast serialization/deserialization
- **Protocol**: Full MCP compliance with JSON-RPC 2.0
- **Search**: Multi-layered resolution with fuzzy matching
- **Performance**: Indexed lookups, cached parsing results

## 🔮 Future Enhancements

- 📝 **Multi-language**: JavaScript, TypeScript, Rust, Go support
- 🌐 **Cross-language**: Track calls between languages
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