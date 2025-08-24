use crate::graph::{CallEdge, CallType, CodeGraph, FunctionNode, Language};
use crate::parser::LanguageParser;
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use tree_sitter::{Node, Parser, Query, QueryCursor, Tree, StreamingIterator};

pub struct PythonParser {
    parser: Parser,
    function_query: Query,
    call_query: Query,
}

impl PythonParser {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_python::LANGUAGE.into())?;

        let function_query = Query::new(
            &tree_sitter_python::LANGUAGE.into(),
            r#"
            (function_definition
                name: (identifier) @name
                parameters: (parameters) @params
            ) @function
            
            (class_definition
                name: (identifier) @class_name
                body: (block
                    (function_definition
                        name: (identifier) @method_name
                        parameters: (parameters) @method_params
                    ) @method
                )
            )
            "#,
        )?;

        let call_query = Query::new(
            &tree_sitter_python::LANGUAGE.into(),
            r#"
            (call
                function: [
                    (identifier) @func_name
                    (attribute
                        attribute: (identifier) @attr_name
                    )
                ]
            ) @call
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
            signature
                .lines()
                .next()
                .unwrap_or("")
                .trim_end_matches(':')
                .to_string()
        } else {
            String::new()
        }
    }

    fn extract_module_path(&self, file_path: &Path) -> Vec<String> {
        let mut components = Vec::new();
        for component in file_path.components() {
            if let Some(s) = component.as_os_str().to_str() {
                if s != "." && s != ".." && !s.ends_with(".py") {
                    components.push(s.to_string());
                }
            }
        }
        if let Some(stem) = file_path.file_stem() {
            if let Some(s) = stem.to_str() {
                if s != "__init__" {
                    components.push(s.to_string());
                }
            }
        }
        components
    }
}

impl LanguageParser for PythonParser {
    fn parse_file(&self, content: &str, file_path: &Path, graph: &mut CodeGraph) -> Result<()> {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_python::LANGUAGE.into())?;
        let tree = parser.parse(content, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse Python file"))?;

        let functions = self.extract_functions(&tree, content, file_path);
        let mut function_map = HashMap::new();

        for func in functions {
            let node_idx = graph.add_function(func.clone());
            function_map.insert(func.name.clone(), node_idx);
        }

        let calls = self.extract_calls(&tree, content);
        
        for (caller_name, call_edge) in calls {
            if let Some(&caller_idx) = function_map.get(&caller_name) {
                for (target_name, &target_idx) in &function_map {
                    if call_edge.call_expression.contains(target_name) {
                        graph.add_call(caller_idx, target_idx, call_edge.clone());
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
            let mut class_name = None;

            for capture in query_match.captures {
                match self.function_query.capture_names()[capture.index as usize] {
                    "name" | "method_name" => {
                        name = capture.node.utf8_text(content.as_bytes()).ok();
                    }
                    "function" | "method" => {
                        node = Some(capture.node);
                    }
                    "class_name" => {
                        class_name = capture.node.utf8_text(content.as_bytes()).ok();
                    }
                    _ => {}
                }
            }

            if let (Some(func_name), Some(func_node)) = (name, node) {
                let full_name = if let Some(cls) = class_name {
                    format!("{}.{}", cls, func_name)
                } else {
                    func_name.to_string()
                };

                functions.push(FunctionNode {
                    name: full_name,
                    file: file_path.to_path_buf(),
                    line: func_node.start_position().row + 1,
                    language: Language::Python,
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

            for capture in query_match.captures {
                match self.call_query.capture_names()[capture.index as usize] {
                    "func_name" | "attr_name" => {
                        call_name = capture.node.utf8_text(content.as_bytes()).ok();
                    }
                    "call" => {
                        call_node = Some(capture.node);
                    }
                    _ => {}
                }
            }

            if let (Some(name), Some(node)) = (call_name, call_node) {
                let mut containing_function = None;
                let mut parent = node.parent();
                
                while let Some(p) = parent {
                    if p.kind() == "function_definition" {
                        if let Some(name_node) = p.child_by_field_name("name") {
                            containing_function = name_node.utf8_text(content.as_bytes()).ok();
                            break;
                        }
                    }
                    parent = p.parent();
                }

                if let Some(func) = containing_function {
                    calls.push((
                        func.to_string(),
                        CallEdge {
                            call_type: CallType::Direct,
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