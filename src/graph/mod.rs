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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SymbolType {
    Function,
    Class,
    Struct,
    Variable,
    Constant,
    Interface,
    Enum,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolNode {
    pub name: String,
    pub file: PathBuf,
    pub line: usize,
    pub language: Language,
    pub signature: String,
    pub module_path: Vec<String>,
    pub symbol_type: SymbolType,
    pub visibility: Option<String>, // public, private, protected, etc.
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationType {
    // Function calls
    DirectCall,
    Import,
    DynamicCall,
    MethodCall,
    
    // Class/struct relationships
    Instantiation,   // Creating instances of classes/structs
    Inheritance,     // Class extends/implements
    FieldAccess,     // Accessing fields of structs/classes
    
    // Variable relationships
    Assignment,      // Variable assignments
    Reference,       // Variable usage/references
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationEdge {
    pub relation_type: RelationType,
    pub line: usize,
    pub expression: String,
}


pub struct CodeGraph {
    pub graph: DiGraph<SymbolNode, RelationEdge>,
    pub symbol_index: HashMap<String, Vec<NodeIndex>>,
    pub file_index: HashMap<PathBuf, Vec<NodeIndex>>,
    pub type_index: HashMap<SymbolType, Vec<NodeIndex>>,
}

impl CodeGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            symbol_index: HashMap::new(),
            file_index: HashMap::new(),
            type_index: HashMap::new(),
        }
    }

    pub fn add_symbol(&mut self, symbol: SymbolNode) -> NodeIndex {
        let name = symbol.name.clone();
        let file = symbol.file.clone();
        let symbol_type = symbol.symbol_type.clone();
        let node_idx = self.graph.add_node(symbol);
        
        self.symbol_index
            .entry(name)
            .or_insert_with(Vec::new)
            .push(node_idx);
            
        self.file_index
            .entry(file)
            .or_insert_with(Vec::new)
            .push(node_idx);
            
        self.type_index
            .entry(symbol_type)
            .or_insert_with(Vec::new)
            .push(node_idx);
            
        node_idx
    }

    pub fn add_relation(&mut self, from: NodeIndex, to: NodeIndex, edge: RelationEdge) {
        self.graph.add_edge(from, to, edge);
    }

    pub fn find_exact(&self, name: &str) -> Option<NodeIndex> {
        self.symbol_index
            .get(name)
            .and_then(|indices| indices.first())
            .copied()
    }

    pub fn find_by_pattern(&self, pattern: &str) -> Vec<NodeIndex> {
        let mut results = Vec::new();
        for (symbol_name, indices) in &self.symbol_index {
            if symbol_name.contains(pattern) {
                results.extend(indices.iter().copied());
            }
        }
        results
    }

    pub fn find_by_type(&self, symbol_type: SymbolType) -> Vec<NodeIndex> {
        self.type_index
            .get(&symbol_type)
            .cloned()
            .unwrap_or_default()
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
        let (nodes, edges): (Vec<SymbolNode>, Vec<(usize, usize, RelationEdge)>) = 
            bincode::deserialize(data)?;
        
        let mut graph = Self::new();
        let mut node_map = HashMap::new();
        
        for node in nodes {
            let idx = graph.add_symbol(node);
            node_map.insert(node_map.len(), idx);
        }
        
        for (from, to, edge) in edges {
            if let (Some(&from_idx), Some(&to_idx)) = 
                (node_map.get(&from), node_map.get(&to)) {
                graph.add_relation(from_idx, to_idx, edge);
            }
        }
        
        Ok(graph)
    }
}