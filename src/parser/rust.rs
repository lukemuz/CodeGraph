use crate::graph::{CallEdge, CallType, CodeGraph, FunctionNode, Language};
use crate::parser::LanguageParser;
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use tree_sitter::{Node, Parser, Query, QueryCursor, Tree, StreamingIterator};

pub struct RustParser {
    parser: Parser,
    function_query: Query,
    call_query: Query,
}

impl RustParser {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_rust::LANGUAGE.into())?;

        let function_query = Query::new(
            &tree_sitter_rust::LANGUAGE.into(),
            r#"
            ; Regular function items
            (function_item
                name: (identifier) @name
                parameters: (parameters) @params
            ) @function

            ; Associated functions and methods in impl blocks
            (impl_item
                type: [
                    (type_identifier) @impl_type
                    (generic_type
                        type: (type_identifier) @impl_type
                    )
                    (scoped_type_identifier
                        name: (type_identifier) @impl_type
                    )
                ]
                body: (declaration_list
                    (function_item
                        name: (identifier) @method_name
                        parameters: (parameters) @method_params
                    ) @method
                )
            )

            ; Functions in trait definitions
            (trait_item
                name: (type_identifier) @trait_name
                body: (declaration_list
                    (function_signature_item
                        name: (identifier) @trait_method_name
                        parameters: (parameters) @trait_method_params
                    ) @trait_method
                )
            )

            ; Functions in trait implementations
            (impl_item
                trait: (type_identifier) @trait_impl_name
                type: (type_identifier) @impl_for_type
                body: (declaration_list
                    (function_item
                        name: (identifier) @trait_impl_method_name
                        parameters: (parameters) @trait_impl_method_params
                    ) @trait_impl_method
                )
            )

            ; Closures assigned to variables
            (let_declaration
                pattern: (identifier) @closure_name
                value: (closure_expression) @closure
            ) @closure_binding

            ; Const functions
            (const_item
                name: (identifier) @const_name
                value: (closure_expression) @const_closure
            ) @const_func
            "#,
        )?;

        let call_query = Query::new(
            &tree_sitter_rust::LANGUAGE.into(),
            r#"
            ; Function calls
            (call_expression
                function: [
                    (identifier) @func_name
                    (field_expression
                        field: (field_identifier) @method_name
                    )
                    (scoped_identifier
                        name: (identifier) @scoped_func_name
                    )
                ]
                arguments: (arguments)
            ) @call

            ; Macro invocations
            (macro_invocation
                macro: [
                    (identifier) @macro_name
                    (scoped_identifier
                        name: (identifier) @scoped_macro_name
                    )
                ]
            ) @macro_call

            ; Method calls using dot notation
            (call_expression
                function: (field_expression
                    field: (field_identifier) @dot_method
                )
            ) @method_call

            ; Await expressions
            (await_expression
                (field_expression
                    field: (field_identifier) @await_method
                )
            ) @await_call
            "#,
        )?;

        Ok(Self {
            parser,
            function_query,
            call_query,
        })
    }

    fn extract_signature(&self, node: &Node, content: &str) -> String {
        if let Ok(signature) = node.utf8_text(content.as_bytes()) {
            let first_line = signature.lines().next().unwrap_or("");
            
            // For Rust, extract up to the opening brace or semicolon
            if let Some(brace_pos) = first_line.find('{') {
                first_line[..brace_pos].trim().to_string()
            } else if let Some(semi_pos) = first_line.find(';') {
                first_line[..semi_pos].trim().to_string()
            } else {
                first_line.trim().to_string()
            }
        } else {
            String::new()
        }
    }

    fn extract_module_path(&self, file_path: &Path) -> Vec<String> {
        let mut components = Vec::new();
        
        // Skip common directory names and build the module path
        for component in file_path.components() {
            if let Some(s) = component.as_os_str().to_str() {
                if s != "." && s != ".." && s != "src" && !s.ends_with(".rs") {
                    components.push(s.to_string());
                }
            }
        }
        
        // Add the file stem unless it's mod.rs or lib.rs or main.rs
        if let Some(stem) = file_path.file_stem() {
            if let Some(s) = stem.to_str() {
                if s != "mod" && s != "lib" && s != "main" {
                    components.push(s.to_string());
                }
            }
        }
        
        components
    }

    fn find_containing_function(&self, node: &Node, content: &str) -> Option<String> {
        let mut parent = node.parent();
        
        while let Some(p) = parent {
            match p.kind() {
                "function_item" => {
                    if let Some(name_node) = p.child_by_field_name("name") {
                        let func_name = name_node.utf8_text(content.as_bytes()).ok()?;
                        
                        // Check if this function is inside an impl block
                        let mut impl_parent = p.parent();
                        while let Some(ip) = impl_parent {
                            if ip.kind() == "impl_item" {
                                // Look for the type being implemented
                                if let Some(type_node) = ip.child_by_field_name("type") {
                                    let type_name = self.extract_type_name(&type_node, content)?;
                                    
                                    // Check if this is a trait implementation
                                    if let Some(trait_node) = ip.child_by_field_name("trait") {
                                        let trait_name = trait_node.utf8_text(content.as_bytes()).ok()?;
                                        return Some(format!("<{} as {}>::{}", type_name, trait_name, func_name));
                                    } else {
                                        return Some(format!("{}::{}", type_name, func_name));
                                    }
                                }
                            }
                            impl_parent = impl_parent.and_then(|p| p.parent());
                        }
                        
                        return Some(func_name.to_string());
                    }
                }
                "closure_expression" => {
                    // Look for the variable it's assigned to
                    if let Some(let_parent) = p.parent() {
                        if let_parent.kind() == "let_declaration" {
                            if let Some(pattern) = let_parent.child_by_field_name("pattern") {
                                return pattern.utf8_text(content.as_bytes()).ok().map(|s| s.to_string());
                            }
                        }
                    }
                }
                _ => {}
            }
            parent = p.parent();
        }
        
        None
    }

    fn extract_type_name(&self, node: &Node, content: &str) -> Option<String> {
        match node.kind() {
            "type_identifier" => node.utf8_text(content.as_bytes()).ok().map(|s| s.to_string()),
            "generic_type" => {
                if let Some(type_node) = node.child_by_field_name("type") {
                    type_node.utf8_text(content.as_bytes()).ok().map(|s| s.to_string())
                } else {
                    None
                }
            }
            "scoped_type_identifier" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    name_node.utf8_text(content.as_bytes()).ok().map(|s| s.to_string())
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

impl LanguageParser for RustParser {
    fn parse_file(&self, content: &str, file_path: &Path, graph: &mut CodeGraph) -> Result<()> {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_rust::LANGUAGE.into())?;
        let tree = parser.parse(content, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse Rust file"))?;

        let functions = self.extract_functions(&tree, content, file_path);
        let mut function_map = HashMap::new();

        for func in functions {
            let node_idx = graph.add_function(func.clone());
            function_map.insert(func.name.clone(), node_idx);
        }

        let calls = self.extract_calls(&tree, content);
        
        for (caller_name, call_edge) in calls {
            if let Some(&caller_idx) = function_map.get(&caller_name) {
                // Try to find the target function
                for (target_name, &target_idx) in &function_map {
                    // Match direct calls, method calls, or scoped calls
                    if call_edge.call_expression == *target_name ||
                       target_name.ends_with(&format!("::{}", call_edge.call_expression)) ||
                       *target_name == format!("Self::{}", call_edge.call_expression) {
                        graph.add_call(caller_idx, target_idx, call_edge.clone());
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    fn extract_functions(&self, tree: &Tree, content: &str, file_path: &Path) -> Vec<FunctionNode> {
        let mut functions = Vec::new();
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&self.function_query, tree.root_node(), content.as_bytes());

        while let Some(query_match) = matches.next() {
            let mut name = None;
            let mut node = None;
            let mut impl_type = None;
            let mut trait_name = None;
            let mut trait_impl_name = None;
            let mut impl_for_type = None;

            for capture in query_match.captures {
                let capture_name = self.function_query.capture_names()[capture.index as usize];
                match capture_name {
                    "name" | "method_name" | "trait_method_name" | "trait_impl_method_name" | "closure_name" | "const_name" => {
                        name = capture.node.utf8_text(content.as_bytes()).ok();
                    }
                    "function" | "method" | "trait_method" | "trait_impl_method" | "closure" | "closure_binding" | "const_func" => {
                        node = Some(capture.node);
                    }
                    "impl_type" => {
                        impl_type = self.extract_type_name(&capture.node, content);
                    }
                    "trait_name" => {
                        trait_name = capture.node.utf8_text(content.as_bytes()).ok();
                    }
                    "trait_impl_name" => {
                        trait_impl_name = capture.node.utf8_text(content.as_bytes()).ok();
                    }
                    "impl_for_type" => {
                        impl_for_type = capture.node.utf8_text(content.as_bytes()).ok();
                    }
                    _ => {}
                }
            }

            if let (Some(func_name), Some(func_node)) = (name, node) {
                let full_name = if let (Some(trait_impl), Some(impl_for)) = (trait_impl_name, impl_for_type) {
                    format!("<{} as {}>::{}", impl_for, trait_impl, func_name)
                } else if let Some(impl_t) = impl_type {
                    format!("{}::{}", impl_t, func_name)
                } else if let Some(trait_n) = trait_name {
                    format!("{}::{}", trait_n, func_name)
                } else {
                    func_name.to_string()
                };

                functions.push(FunctionNode {
                    name: full_name,
                    file: file_path.to_path_buf(),
                    line: func_node.start_position().row + 1,
                    language: Language::Rust,
                    signature: self.extract_signature(&func_node, content),
                    module_path: self.extract_module_path(file_path),
                });
            }
        }

        functions
    }

    fn extract_calls(&self, tree: &Tree, content: &str) -> Vec<(String, CallEdge)> {
        let mut calls = Vec::new();
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&self.call_query, tree.root_node(), content.as_bytes());

        while let Some(query_match) = matches.next() {
            let mut call_name = None;
            let mut call_node = None;
            let mut is_macro = false;

            for capture in query_match.captures {
                let capture_name = self.call_query.capture_names()[capture.index as usize];
                match capture_name {
                    "func_name" | "method_name" | "scoped_func_name" | "dot_method" | "await_method" => {
                        call_name = capture.node.utf8_text(content.as_bytes()).ok();
                    }
                    "macro_name" | "scoped_macro_name" => {
                        call_name = capture.node.utf8_text(content.as_bytes()).ok();
                        is_macro = true;
                    }
                    "call" | "macro_call" | "method_call" | "await_call" => {
                        call_node = Some(capture.node);
                    }
                    _ => {}
                }
            }

            if let (Some(name), Some(node)) = (call_name, call_node) {
                if let Some(func) = self.find_containing_function(&node, content) {
                    let call_type = if is_macro {
                        CallType::Dynamic // Could add a Macro variant
                    } else if name.contains("::") {
                        CallType::Direct
                    } else {
                        CallType::Direct
                    };

                    calls.push((
                        func,
                        CallEdge {
                            call_type,
                            line: node.start_position().row + 1,
                            call_expression: name.to_string(),
                        },
                    ));
                }
            }
        }

        calls
    }
}