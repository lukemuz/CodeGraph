use crate::graph::{CallEdge, CallType, CodeGraph, FunctionNode, Language};
use crate::parser::LanguageParser;
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use tree_sitter::{Node, Parser, Query, QueryCursor, Tree, StreamingIterator};

pub struct JavaScriptParser {
    parser: Parser,
    function_query: Query,
    call_query: Query,
    language: Language,
}

impl JavaScriptParser {
    pub fn new(is_typescript: bool) -> Result<Self> {
        let mut parser = Parser::new();
        let language = if is_typescript {
            Language::TypeScript
        } else {
            Language::JavaScript
        };

        let ts_language = if is_typescript {
            tree_sitter_typescript::LANGUAGE_TSX.into()
        } else {
            tree_sitter_javascript::LANGUAGE.into()
        };

        parser.set_language(&ts_language)?;

        let function_query = Query::new(
            &ts_language,
            r#"
            ; Regular function declarations
            (function_declaration
                name: (identifier) @name
            ) @function

            ; Arrow functions assigned to variables
            (variable_declarator
                name: (identifier) @name
                value: (arrow_function)
            ) @arrow_function

            ; Function expressions assigned to variables
            (variable_declarator
                name: (identifier) @name
                value: (function_expression)
            ) @func_expression

            ; Methods in classes
            (method_definition
                name: (property_identifier) @method_name
            ) @method
            "#,
        )?;

        let call_query = Query::new(
            &ts_language,
            r#"
            ; Regular function calls
            (call_expression
                function: [
                    (identifier) @func_name
                    (member_expression
                        property: (property_identifier) @method_name
                    )
                ]
                arguments: (arguments)
            ) @call

            ; New expressions (constructor calls)
            (new_expression
                constructor: (identifier) @class_name
                arguments: (arguments)?
            ) @new_call

            ; Await expressions
            (await_expression
                (call_expression
                    function: [
                        (identifier) @async_func
                        (member_expression
                            property: (property_identifier) @async_method
                        )
                    ]
                )
            ) @await_call
            "#,
        )?;

        Ok(Self {
            parser,
            function_query,
            call_query,
            language,
        })
    }

    fn extract_signature(&self, node: &Node, content: &str) -> String {
        if let Ok(signature) = node.utf8_text(content.as_bytes()) {
            let first_line = signature.lines().next().unwrap_or("");
            
            // Clean up the signature
            if first_line.contains("{") {
                first_line.split("{").next().unwrap_or(first_line).trim().to_string()
            } else {
                first_line.trim().to_string()
            }
        } else {
            String::new()
        }
    }

    fn extract_module_path(&self, file_path: &Path) -> Vec<String> {
        let mut components = Vec::new();
        for component in file_path.components() {
            if let Some(s) = component.as_os_str().to_str() {
                if s != "." && s != ".." && !s.ends_with(".js") && !s.ends_with(".ts") && !s.ends_with(".jsx") && !s.ends_with(".tsx") {
                    components.push(s.to_string());
                }
            }
        }
        if let Some(stem) = file_path.file_stem() {
            if let Some(s) = stem.to_str() {
                if s != "index" {
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
                "function_declaration" | "function_expression" => {
                    if let Some(name_node) = p.child_by_field_name("name") {
                        return name_node.utf8_text(content.as_bytes()).ok().map(|s| s.to_string());
                    }
                }
                "variable_declarator" => {
                    // Check if the value is a function
                    if let Some(value) = p.child_by_field_name("value") {
                        if value.kind() == "arrow_function" || value.kind() == "function_expression" {
                            if let Some(name_node) = p.child_by_field_name("name") {
                                return name_node.utf8_text(content.as_bytes()).ok().map(|s| s.to_string());
                            }
                        }
                    }
                }
                "method_definition" => {
                    if let Some(name_node) = p.child_by_field_name("name") {
                        let method_name = name_node.utf8_text(content.as_bytes()).ok()?;
                        
                        // Look for the containing class
                        let mut class_parent = p.parent();
                        while let Some(cp) = class_parent {
                            if cp.kind() == "class_declaration" {
                                if let Some(class_name_node) = cp.child_by_field_name("name") {
                                    let class_name = class_name_node.utf8_text(content.as_bytes()).ok()?;
                                    return Some(format!("{}.{}", class_name, method_name));
                                }
                            }
                            class_parent = cp.parent();
                        }
                        
                        return Some(method_name.to_string());
                    }
                }
                _ => {}
            }
            parent = p.parent();
        }
        
        None
    }
}

impl LanguageParser for JavaScriptParser {
    fn parse_file(&self, content: &str, file_path: &Path, graph: &mut CodeGraph) -> Result<()> {
        let mut parser = Parser::new();
        let ts_language = if self.language == Language::TypeScript {
            tree_sitter_typescript::LANGUAGE_TSX.into()
        } else {
            tree_sitter_javascript::LANGUAGE.into()
        };
        parser.set_language(&ts_language)?;
        let tree = parser.parse(content, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse JavaScript/TypeScript file"))?;

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
                    if call_edge.call_expression == *target_name || 
                       call_edge.call_expression.ends_with(&format!(".{}", target_name)) {
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

            for capture in query_match.captures {
                let capture_name = self.function_query.capture_names()[capture.index as usize];
                match capture_name {
                    "name" | "method_name" => {
                        name = capture.node.utf8_text(content.as_bytes()).ok();
                    }
                    "function" | "arrow_function" | "func_expression" | "method" => {
                        node = Some(capture.node);
                    }
                    _ => {}
                }
            }

            let func_name = name.map(|n| n.to_string());

            if let (Some(func_name), Some(func_node)) = (func_name, node) {
                functions.push(FunctionNode {
                    name: func_name,
                    file: file_path.to_path_buf(),
                    line: func_node.start_position().row + 1,
                    language: self.language.clone(),
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
            let mut is_new = false;

            for capture in query_match.captures {
                let capture_name = self.call_query.capture_names()[capture.index as usize];
                match capture_name {
                    "func_name" | "method_name" | "async_func" | "async_method" => {
                        call_name = capture.node.utf8_text(content.as_bytes()).ok();
                    }
                    "class_name" => {
                        call_name = capture.node.utf8_text(content.as_bytes()).ok();
                        is_new = true;
                    }
                    "call" | "new_call" | "await_call" => {
                        call_node = Some(capture.node);
                    }
                    _ => {}
                }
            }

            if let (Some(name), Some(node)) = (call_name, call_node) {
                if let Some(func) = self.find_containing_function(&node, content) {
                    let call_type = if is_new {
                        CallType::Direct // Could add a Constructor variant
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