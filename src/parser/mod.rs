pub mod python;

use crate::graph::{CallEdge, CodeGraph, FunctionNode, Language};
use anyhow::Result;
use std::path::Path;
use tree_sitter::Tree;

pub trait LanguageParser {
    fn parse_file(&self, content: &str, file_path: &Path, graph: &mut CodeGraph) -> Result<()>;
    fn extract_functions(&self, tree: &Tree, content: &str, file_path: &Path) -> Vec<FunctionNode>;
    fn extract_calls(&self, tree: &Tree, content: &str) -> Vec<(String, CallEdge)>;
}

pub struct ParserManager {
    python_parser: python::PythonParser,
}

impl ParserManager {
    pub fn new() -> Result<Self> {
        Ok(Self {
            python_parser: python::PythonParser::new()?,
        })
    }

    pub fn parse_file(&self, file_path: &Path, content: &str, graph: &mut CodeGraph) -> Result<()> {
        let extension = file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");

        match extension {
            "py" => self.python_parser.parse_file(content, file_path, graph),
            _ => Ok(()), // Skip non-Python files for now
        }
    }

    pub fn get_language(file_path: &Path) -> Option<Language> {
        let extension = file_path
            .extension()
            .and_then(|ext| ext.to_str())?;

        match extension {
            "py" => Some(Language::Python),
            _ => None,
        }
    }
}