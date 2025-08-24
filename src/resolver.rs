use crate::graph::CodeGraph;
use anyhow::Result;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FunctionRef {
    pub name: String,
    pub file: String,
    pub line: usize,
    pub signature: String,
    pub confidence: f64,
}

pub struct FunctionResolver {
    matcher: SkimMatcherV2,
}

impl FunctionResolver {
    pub fn new() -> Self {
        Self {
            matcher: SkimMatcherV2::default(),
        }
    }

    pub fn resolve_function_reference(
        &self,
        query: &str,
        graph: &CodeGraph,
        scope: Option<&Path>,
    ) -> Result<Vec<FunctionRef>> {
        let mut candidates = Vec::new();

        if let Some(exact_node) = graph.find_exact(query) {
            if let Some(function) = graph.graph.node_weight(exact_node) {
                candidates.push(FunctionRef {
                    name: function.name.clone(),
                    file: function.file.to_string_lossy().to_string(),
                    line: function.line,
                    signature: function.signature.clone(),
                    confidence: 1.0,
                });
            }
        }

        let pattern_matches = graph.find_by_pattern(query);
        for node_idx in pattern_matches {
            if let Some(function) = graph.graph.node_weight(node_idx) {
                if let Some(score) = self.matcher.fuzzy_match(&function.name, query) {
                    let confidence = (score as f64) / 100.0;
                    if confidence > 0.3 {
                        candidates.push(FunctionRef {
                            name: function.name.clone(),
                            file: function.file.to_string_lossy().to_string(),
                            line: function.line,
                            signature: function.signature.clone(),
                            confidence,
                        });
                    }
                }
            }
        }

        if candidates.is_empty() {
            candidates.extend(self.ripgrep_search(query, scope)?);
        }

        candidates.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        candidates.truncate(10);
        Ok(candidates)
    }

    fn ripgrep_search(&self, query: &str, scope: Option<&Path>) -> Result<Vec<FunctionRef>> {
        let mut results = Vec::new();
        let search_path = scope.unwrap_or_else(|| Path::new("."));

        let patterns = vec![
            Regex::new(&format!(r"def\s+{}\s*\(", regex::escape(query)))?,
            Regex::new(&format!(r"function\s+{}\s*\(", regex::escape(query)))?,
            Regex::new(&format!(r"const\s+{}\s*=", regex::escape(query)))?,
            Regex::new(&format!(r"let\s+{}\s*=", regex::escape(query)))?,
            Regex::new(&format!(r"var\s+{}\s*=", regex::escape(query)))?,
            Regex::new(&format!(r"{}\s*\(", regex::escape(query)))?,
        ];

        for entry in WalkDir::new(search_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|ext| matches!(ext.to_str(), Some("py")))
                    .unwrap_or(false)
            })
        {
            if let Ok(content) = fs::read_to_string(entry.path()) {
                for (line_num, line) in content.lines().enumerate() {
                    for pattern in &patterns {
                        if pattern.is_match(line) {
                            let confidence = if line.contains(&format!("def {}", query))
                                || line.contains(&format!("function {}", query))
                            {
                                0.9
                            } else if line.contains(query) {
                                0.6
                            } else {
                                0.3
                            };

                            results.push(FunctionRef {
                                name: query.to_string(),
                                file: entry.path().to_string_lossy().to_string(),
                                line: line_num + 1,
                                signature: line.trim().to_string(),
                                confidence,
                            });
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    pub fn find_functions_in_scope(
        &self,
        graph: &CodeGraph,
        scope: &Path,
        pattern: Option<&str>,
    ) -> Vec<FunctionRef> {
        let mut results = Vec::new();

        for (_, node_indices) in &graph.file_index {
            for &node_idx in node_indices {
                if let Some(function) = graph.graph.node_weight(node_idx) {
                    if function.file.starts_with(scope) {
                        let matches = if let Some(p) = pattern {
                            function.name.contains(p)
                                || self.matcher.fuzzy_match(&function.name, p).is_some()
                        } else {
                            true
                        };

                        if matches {
                            results.push(FunctionRef {
                                name: function.name.clone(),
                                file: function.file.to_string_lossy().to_string(),
                                line: function.line,
                                signature: function.signature.clone(),
                                confidence: 1.0,
                            });
                        }
                    }
                }
            }
        }

        results
    }

    pub fn rank_by_popularity(&self, candidates: &mut [FunctionRef], graph: &CodeGraph) {
        let mut popularity_scores = HashMap::new();

        for (name, indices) in &graph.function_index {
            for &idx in indices {
                let caller_count = graph.get_callers(idx).len();
                popularity_scores.insert(name.clone(), caller_count);
            }
        }

        for candidate in candidates.iter_mut() {
            if let Some(&popularity) = popularity_scores.get(&candidate.name) {
                candidate.confidence *= 1.0 + (popularity as f64 * 0.1);
            }
        }

        candidates.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
}

