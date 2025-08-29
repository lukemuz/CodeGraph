# Changelog

All notable changes to this project will be documented in this file.

## [0.3.1] - 2025-08-29

### 🚀 Performance & Simplicity

#### Removed Lazy Refresh System
- **REMOVED**: Complex sampling-based freshness checking (161 lines of code eliminated)
- **IMPROVED**: Always rebuild index on MCP requests - simpler and more reliable
- **PERFORMANCE**: Even large repos (34k+ files) rebuild in under 2 seconds
- **REMOVED**: `--auto-refresh` and `--refresh-interval` CLI flags 
- **RESULT**: Dramatically simpler codebase with better performance than "optimized" version

**Rationale**: Timing analysis showed timestamp checking took 95% as long as full rebuild, making the complexity pointless. The "always rebuild" approach is faster, simpler, and always serves fresh data.

---

## [0.3.0] - 2025-08-25

### 🎉 Major Features

#### Universal Symbol Support
- **BREAKING**: Replaced `FunctionNode` with `SymbolNode` to support all code symbols
- **NEW**: Support for classes, structs, variables, constants, interfaces, and enums
- **NEW**: Enhanced `SymbolType` enum with comprehensive symbol classification
- **NEW**: `visibility` field for public/private/protected symbol access

#### Enhanced Relationship Tracking  
- **BREAKING**: Replaced `CallEdge` with `RelationEdge` for richer relationships
- **NEW**: `RelationType` enum supporting:
  - `DirectCall`, `MethodCall`, `DynamicCall` (function calls)
  - `Instantiation` (object/struct creation)  
  - `Inheritance` (class extends/implements)
  - `FieldAccess` (property/field access)
  - `Assignment`, `Reference` (variable usage)

#### Improved Indexing System
- **NEW**: `symbol_index` replaces `function_index` for all symbols
- **NEW**: `type_index` for efficient symbol type lookups
- **NEW**: `find_by_type()` method for type-specific searches

### 🚀 MCP Tool Enhancements

#### Better LLM Discoverability
- **IMPROVED**: Professional tool descriptions without overselling
- **IMPROVED**: Balanced priority scores (1.0-1.2) competitive with built-in tools
- **IMPROVED**: Clear capability explanations for better adoption

#### Enhanced Tool Functionality
- **IMPROVED**: `navigate` tool now works with all symbol types
- **IMPROVED**: `find` tool searches across all symbol types with fuzzy matching
- **IMPROVED**: `impact` tool analyzes dependencies for any symbol type
- **NEW**: Updated parameter descriptions to reflect universal symbol support

### 🔧 Implementation Changes

#### Parser Updates
- **UPDATED**: All language parsers (Python, JavaScript/TypeScript, Rust) use new symbol system
- **IMPROVED**: Better relationship detection (instantiation, inheritance, field access)
- **IMPROVED**: Enhanced signature extraction for all symbol types

#### API Changes
- **BREAKING**: `add_function()` → `add_symbol()`
- **BREAKING**: `add_call()` → `add_relation()`  
- **BREAKING**: `extract_functions()` → `extract_symbols()`
- **BREAKING**: `extract_calls()` → `extract_relations()`
- **REMOVED**: Legacy type aliases for clean codebase

### 🧪 Testing & Quality

#### Comprehensive Test Suite
- **NEW**: `examples/test_new_symbols.rs` - Symbol system verification
- **NEW**: `examples/test_mcp_tools.rs` - MCP tool functionality tests  
- **NEW**: `test_summary.sh` - Complete integration testing
- **IMPROVED**: Better error handling and validation

#### Performance & Reliability
- **IMPROVED**: More efficient symbol lookups with type indexing
- **IMPROVED**: Better memory usage with optimized data structures
- **FIXED**: All compilation warnings resolved

### 📚 Documentation

#### Updated Tool Descriptions
- **IMPROVED**: Clear, professional MCP tool descriptions
- **IMPROVED**: Better examples and use case explanations
- **IMPROVED**: Accurate capability descriptions without hype

### 🔄 Migration Guide

For users upgrading from v0.2.x:

1. **Index Rebuild Required**: Existing indexes need to be rebuilt with `codegraph index . --force`
2. **API Changes**: If using CodeGraph as a library:
   - Replace `FunctionNode` with `SymbolNode`  
   - Replace `CallEdge` with `RelationEdge`
   - Update method calls: `add_function()` → `add_symbol()`
3. **MCP Tools**: No changes needed - tools work the same but with expanded capabilities

### 🏗️ Architecture

The new symbol-based architecture provides:
- **Universal Coverage**: Track relationships between any code symbols
- **Semantic Understanding**: Distinguish between calls, instantiations, inheritance, etc.
- **Type-Aware Operations**: Search and analyze by symbol type
- **Enhanced Analysis**: Richer relationship data for better insights
- **Future-Proof**: Extensible for new languages and symbol types

---

## [0.2.0] - Previous Release
- Initial MCP server implementation
- Function-only relationship tracking  
- Basic navigate/find/impact operations

## [0.1.0] - Initial Release  
- Core graph functionality
- Multi-language parsing (Python, JavaScript, TypeScript, Rust)
- Basic indexing and CLI tools