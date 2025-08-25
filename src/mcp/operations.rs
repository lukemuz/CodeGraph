use crate::graph::{CodeGraph, Language};
use crate::mcp::{FunctionInfo, NavigateResult, ImpactResult, FindResult};
use crate::resolver::{FunctionResolver, FunctionRef};
use anyhow::Result;
use petgraph::graph::NodeIndex;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub struct OperationHandler {
    resolver: FunctionResolver,
}

impl OperationHandler {
    pub fn new() -> Self {
        Self {
            resolver: FunctionResolver::new(),
        }
    }

    pub fn navigate(
        &self,
        graph: &CodeGraph,
        function_name: &str,
        depth: Option<usize>,
    ) -> Result<NavigateResult> {
        let candidates = self.resolver.resolve_function_reference(function_name, graph, None)?;
        
        if candidates.is_empty() {
            return Err(anyhow::anyhow!("Function '{}' not found", function_name));
        }

        let best_match = &candidates[0];
        
        if let Some(node_idx) = graph.symbol_index.get(&best_match.name).and_then(|v| v.first()) {
            let function_node = graph.graph.node_weight(*node_idx).unwrap();
            
            let calls = self.get_function_calls(graph, *node_idx, depth.unwrap_or(1));
            let called_by = self.get_function_callers(graph, *node_idx, depth.unwrap_or(1));
            let siblings = self.get_function_siblings(graph, *node_idx);

            let function_info = FunctionInfo {
                name: function_node.name.clone(),
                file: function_node.file.to_string_lossy().to_string(),
                line: function_node.line,
                signature: function_node.signature.clone(),
                language: self.language_to_string(&function_node.language),
                module_path: function_node.module_path.clone(),
            };

            let summary = self.generate_navigate_summary(&function_info, &calls, &called_by, &siblings);

            Ok(NavigateResult {
                function: function_info,
                calls,
                called_by,
                siblings,
                summary,
            })
        } else {
            Err(anyhow::anyhow!("Function node not found in graph"))
        }
    }

    pub fn analyze_impact(
        &self,
        graph: &CodeGraph,
        function_name: &str,
        include_tests: bool,
    ) -> Result<ImpactResult> {
        let candidates = self.resolver.resolve_function_reference(function_name, graph, None)?;
        
        if candidates.is_empty() {
            return Err(anyhow::anyhow!("Function '{}' not found", function_name));
        }

        let best_match = &candidates[0];
        
        if let Some(node_idx) = graph.symbol_index.get(&best_match.name).and_then(|v| v.first()) {
            let direct_callers = self.get_function_callers(graph, *node_idx, 1);
            let transitive_impact = self.get_transitive_impact(graph, *node_idx);
            
            let mut affected_files = HashSet::new();
            let mut test_files = HashSet::new();

            for caller in &direct_callers {
                let file_path = PathBuf::from(&caller.file);
                affected_files.insert(file_path.clone());
                
                if include_tests && self.is_test_file(&file_path) {
                    test_files.insert(file_path);
                }
            }

            for impact in &transitive_impact {
                let file_path = PathBuf::from(&impact.file);
                affected_files.insert(file_path.clone());
                
                if include_tests && self.is_test_file(&file_path) {
                    test_files.insert(file_path);
                }
            }

            let risk_level = self.assess_risk_level(&direct_callers, &transitive_impact);
            let summary = self.generate_impact_summary(
                function_name, 
                &direct_callers, 
                &transitive_impact, 
                &risk_level
            );

            Ok(ImpactResult {
                direct_callers,
                transitive_impact,
                affected_files: affected_files.into_iter().collect(),
                test_files: test_files.into_iter().collect(),
                risk_level,
                summary,
            })
        } else {
            Err(anyhow::anyhow!("Function node not found in graph"))
        }
    }

    pub fn find_functions(
        &self,
        graph: &CodeGraph,
        query: &str,
        scope: Option<&Path>,
    ) -> Result<FindResult> {
        let matches = self.resolver.resolve_function_reference(query, graph, scope)?;
        
        let mut grouped_by_file = HashMap::new();
        for func_ref in &matches {
            let file_path = PathBuf::from(&func_ref.file);
            grouped_by_file
                .entry(file_path)
                .or_insert_with(Vec::new)
                .push(func_ref.clone());
        }

        let summary = self.generate_find_summary(query, &matches, scope);

        Ok(FindResult {
            matches,
            grouped_by_file,
            summary,
        })
    }

    fn get_function_calls(&self, graph: &CodeGraph, node_idx: NodeIndex, depth: usize) -> Vec<FunctionRef> {
        let mut results = Vec::new();
        let mut visited = HashSet::new();
        self.collect_calls_recursive(graph, node_idx, depth, 0, &mut visited, &mut results);
        results
    }

    fn get_function_callers(&self, graph: &CodeGraph, node_idx: NodeIndex, depth: usize) -> Vec<FunctionRef> {
        let mut results = Vec::new();
        let mut visited = HashSet::new();
        self.collect_callers_recursive(graph, node_idx, depth, 0, &mut visited, &mut results);
        results
    }

    fn get_function_siblings(&self, graph: &CodeGraph, node_idx: NodeIndex) -> Vec<FunctionRef> {
        graph.get_siblings(node_idx)
            .into_iter()
            .filter_map(|idx| {
                graph.graph.node_weight(idx).map(|node| FunctionRef {
                    name: node.name.clone(),
                    file: node.file.to_string_lossy().to_string(),
                    line: node.line,
                    signature: node.signature.clone(),
                    confidence: 1.0,
                })
            })
            .collect()
    }

    fn collect_calls_recursive(
        &self,
        graph: &CodeGraph,
        node_idx: NodeIndex,
        max_depth: usize,
        current_depth: usize,
        visited: &mut HashSet<NodeIndex>,
        results: &mut Vec<FunctionRef>,
    ) {
        if current_depth >= max_depth || visited.contains(&node_idx) {
            return;
        }

        visited.insert(node_idx);
        
        for callee_idx in graph.get_callees(node_idx) {
            if let Some(node) = graph.graph.node_weight(callee_idx) {
                results.push(FunctionRef {
                    name: node.name.clone(),
                    file: node.file.to_string_lossy().to_string(),
                    line: node.line,
                    signature: node.signature.clone(),
                    confidence: 1.0,
                });

                if current_depth + 1 < max_depth {
                    self.collect_calls_recursive(graph, callee_idx, max_depth, current_depth + 1, visited, results);
                }
            }
        }
    }

    fn collect_callers_recursive(
        &self,
        graph: &CodeGraph,
        node_idx: NodeIndex,
        max_depth: usize,
        current_depth: usize,
        visited: &mut HashSet<NodeIndex>,
        results: &mut Vec<FunctionRef>,
    ) {
        if current_depth >= max_depth || visited.contains(&node_idx) {
            return;
        }

        visited.insert(node_idx);
        
        for caller_idx in graph.get_callers(node_idx) {
            if let Some(node) = graph.graph.node_weight(caller_idx) {
                results.push(FunctionRef {
                    name: node.name.clone(),
                    file: node.file.to_string_lossy().to_string(),
                    line: node.line,
                    signature: node.signature.clone(),
                    confidence: 1.0,
                });

                if current_depth + 1 < max_depth {
                    self.collect_callers_recursive(graph, caller_idx, max_depth, current_depth + 1, visited, results);
                }
            }
        }
    }

    fn get_transitive_impact(&self, graph: &CodeGraph, node_idx: NodeIndex) -> Vec<FunctionRef> {
        let mut results = Vec::new();
        let mut visited = HashSet::new();
        self.collect_callers_recursive(graph, node_idx, 3, 0, &mut visited, &mut results);
        results
    }

    fn is_test_file(&self, file_path: &Path) -> bool {
        let path_str = file_path.to_string_lossy().to_lowercase();
        path_str.contains("test") || path_str.contains("spec") || 
        file_path.file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.starts_with("test_") || name.ends_with("_test.py") || 
                       name.ends_with(".test.js") || name.ends_with(".spec.js"))
            .unwrap_or(false)
    }

    fn assess_risk_level(&self, direct_callers: &[FunctionRef], transitive_impact: &[FunctionRef]) -> String {
        let total_impact = direct_callers.len() + transitive_impact.len();
        
        match total_impact {
            0..=2 => "low".to_string(),
            3..=10 => "medium".to_string(),
            _ => "high".to_string(),
        }
    }

    fn generate_navigate_summary(
        &self,
        function: &FunctionInfo,
        calls: &[FunctionRef],
        called_by: &[FunctionRef],
        siblings: &[FunctionRef],
    ) -> String {
        format!(
            "Function '{}' at {}:{} calls {} functions, is called by {} functions, and has {} siblings in the same file.",
            function.name,
            function.file,
            function.line,
            calls.len(),
            called_by.len(),
            siblings.len()
        )
    }

    fn generate_impact_summary(
        &self,
        function_name: &str,
        direct_callers: &[FunctionRef],
        transitive_impact: &[FunctionRef],
        risk_level: &str,
    ) -> String {
        format!(
            "Changing '{}' would directly affect {} functions and transitively impact {} functions. Risk level: {}.",
            function_name,
            direct_callers.len(),
            transitive_impact.len(),
            risk_level
        )
    }

    fn generate_find_summary(&self, query: &str, matches: &[FunctionRef], scope: Option<&Path>) -> String {
        let scope_str = if let Some(s) = scope {
            format!(" in {}", s.display())
        } else {
            "".to_string()
        };
        
        format!(
            "Found {} functions matching '{}'{}.{}",
            matches.len(),
            query,
            scope_str,
            if matches.len() > 10 { " (showing top 10)" } else { "" }
        )
    }

    fn language_to_string(&self, language: &Language) -> String {
        match language {
            Language::Python => "Python".to_string(),
            Language::JavaScript => "JavaScript".to_string(),
            Language::TypeScript => "TypeScript".to_string(),
            Language::Rust => "Rust".to_string(),
        }
    }
}