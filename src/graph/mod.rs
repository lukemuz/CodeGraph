use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Language {
    Python,
    JavaScript,
    TypeScript,
    Rust,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionNode {
    pub name: String,
    pub file: PathBuf,
    pub line: usize,
    pub language: Language,
    pub signature: String,
    pub module_path: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CallType {
    Direct,
    Import,
    Dynamic,
    Method,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallEdge {
    pub call_type: CallType,
    pub line: usize,
    pub call_expression: String,
}

pub struct CodeGraph {
    pub graph: DiGraph<FunctionNode, CallEdge>,
    pub function_index: HashMap<String, Vec<NodeIndex>>,
    pub file_index: HashMap<PathBuf, Vec<NodeIndex>>,
}

impl CodeGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            function_index: HashMap::new(),
            file_index: HashMap::new(),
        }
    }

    pub fn add_function(&mut self, function: FunctionNode) -> NodeIndex {
        let name = function.name.clone();
        let file = function.file.clone();
        let node_idx = self.graph.add_node(function);
        
        self.function_index
            .entry(name)
            .or_insert_with(Vec::new)
            .push(node_idx);
            
        self.file_index
            .entry(file)
            .or_insert_with(Vec::new)
            .push(node_idx);
            
        node_idx
    }

    pub fn add_call(&mut self, from: NodeIndex, to: NodeIndex, edge: CallEdge) {
        self.graph.add_edge(from, to, edge);
    }

    pub fn find_exact(&self, name: &str) -> Option<NodeIndex> {
        self.function_index
            .get(name)
            .and_then(|indices| indices.first())
            .copied()
    }

    pub fn find_by_pattern(&self, pattern: &str) -> Vec<NodeIndex> {
        let mut results = Vec::new();
        for (func_name, indices) in &self.function_index {
            if func_name.contains(pattern) {
                results.extend(indices.iter().copied());
            }
        }
        results
    }

    pub fn get_callers(&self, node: NodeIndex) -> Vec<NodeIndex> {
        self.graph
            .neighbors_directed(node, petgraph::Direction::Incoming)
            .collect()
    }

    pub fn get_callees(&self, node: NodeIndex) -> Vec<NodeIndex> {
        self.graph
            .neighbors_directed(node, petgraph::Direction::Outgoing)
            .collect()
    }

    pub fn get_siblings(&self, node: NodeIndex) -> Vec<NodeIndex> {
        if let Some(function) = self.graph.node_weight(node) {
            self.file_index
                .get(&function.file)
                .map(|indices| {
                    indices
                        .iter()
                        .filter(|&&idx| idx != node)
                        .copied()
                        .collect()
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        }
    }

    pub fn serialize(&self) -> Result<Vec<u8>, bincode::Error> {
        let nodes: Vec<_> = self.graph.node_weights().cloned().collect();
        let edges: Vec<_> = self.graph
            .edge_indices()
            .map(|e| {
                let (a, b) = self.graph.edge_endpoints(e).unwrap();
                (a.index(), b.index(), self.graph[e].clone())
            })
            .collect();
        
        bincode::serialize(&(nodes, edges))
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, bincode::Error> {
        let (nodes, edges): (Vec<FunctionNode>, Vec<(usize, usize, CallEdge)>) = 
            bincode::deserialize(data)?;
        
        let mut graph = Self::new();
        let mut node_map = HashMap::new();
        
        for node in nodes {
            let idx = graph.add_function(node);
            node_map.insert(node_map.len(), idx);
        }
        
        for (from, to, edge) in edges {
            if let (Some(&from_idx), Some(&to_idx)) = 
                (node_map.get(&from), node_map.get(&to)) {
                graph.add_call(from_idx, to_idx, edge);
            }
        }
        
        Ok(graph)
    }
}