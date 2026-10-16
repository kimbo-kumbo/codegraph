//CodeGraph Parser — tree-sitter парсеры для Go, Python, Rust.
pub mod go;
pub mod python;
pub mod rust;
pub mod walker;
pub mod complexity;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use codegraph_ir::{Graph, Language};
use thiserror::Error;


#[derive(Debug, Error)]
pub enum ParseError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("unsupported file extension: {0}")]
    UnsupportedExtension(String),

    #[error("failed to load tree-sitter language for {0}")]
    LanguageLoad(Language),

    #[error("tree-sitter failed to parse: {0}")]
    Parse(String),
}

pub type Result<T> = std::result::Result<T, ParseError>;

pub fn detect_language(path: &Path) -> Option<Language> {
    let ext = path.extension()?.to_str()?;
    Language::from_extension(ext)
}

pub fn parse_file(path: &Path, base_dir: &Path) -> Result<Graph> {
    let language = detect_language(path).ok_or_else(|| {
        ParseError::UnsupportedExtension(
            path.extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string(),
        )
    })?;

    let source = std::fs::read_to_string(path)?;
    let rel_path = path
        .strip_prefix(base_dir)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();

    match language {
        Language::Go => {
            let (mut graph, index) = go::parse_declarations(&source, &rel_path)?;
            let calls = go::parse_calls(&source, &rel_path, &index)?;
            for edge in calls.edges {
                graph.add_edge(edge);
            }
            graph.compute_fan_metrics();
            Ok(graph)
        }
        Language::Python => {
            let (mut graph, index) = python::parse_declarations(&source, &rel_path)?;
            let calls = python::parse_calls(&source, &rel_path, &index)?;
            for edge in calls.edges {
                graph.add_edge(edge);
            }
            graph.compute_fan_metrics();
            Ok(graph)
        }
        Language::Rust => {
            let (mut graph, index) = rust::parse_declarations(&source, &rel_path)?;
            let calls = rust::parse_calls(&source, &rel_path, &index)?;
            for edge in calls.edges {
                graph.add_edge(edge);
            }
            graph.compute_fan_metrics();
            Ok(graph)
        },
    }
}

pub fn parse_dir(root: &Path) -> Result<Graph> {
    let mut graph = Graph::new();
    let mut global_index: HashMap<String, String> = HashMap::new();

    let mut files: Vec<(PathBuf, String, Language)> = Vec::new();

    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| !is_ignored(e.path()))
    {
        let entry = entry.map_err(|e| {
            ParseError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
        })?;

        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(lang) = detect_language(path) else {
            continue;
        };

        match std::fs::read_to_string(path) {
            Ok(source) => files.push((path.to_path_buf(), source, lang)),
            Err(e) => eprintln!("[warn] cannot read {}: {}", path.display(), e),
        }
    }

    for (path, source, lang) in &files {
        let rel_path = rel_path(path, root);
        let result = match lang {
            Language::Go => go::parse_declarations(source, &rel_path),
            Language::Python => python::parse_declarations(source, &rel_path),
            Language::Rust => rust::parse_declarations(source, &rel_path),
        };

        match result {
            Ok((sub_graph, index)) => {
                for node in sub_graph.nodes.into_values() {
                    graph.add_node(node);
                }
                for (name, id) in index {
                    global_index.entry(name).or_insert(id);
                }
            }
            Err(e) => eprintln!("[warn] {}: {}", path.display(), e),
        }
    }

    for (path, source, lang) in &files {
        let rel_path = rel_path(path, root);
        let result = match lang {
            Language::Go => go::parse_calls(source, &rel_path, &global_index),
            Language::Python => python::parse_calls(source, &rel_path, &global_index),
            Language::Rust => rust::parse_calls(source, &rel_path, &global_index),
        };

        match result {
            Ok(sub_graph) => {
                for edge in sub_graph.edges {
                    graph.add_edge(edge);
                }
            }
            Err(e) => eprintln!("[warn] {}: {}", path.display(), e),
        }
    }

    graph.compute_fan_metrics();

    eprintln!(
        "[codegraph-parser] parsed {} files, {} nodes, {} edges from {}",
        files.len(),
        graph.node_count(),
        graph.edge_count(),
        root.display()
    );

    Ok(graph)
}

pub fn list_source_files(root: &Path) -> Vec<PathBuf> {
    walkdir::WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| !is_ignored(e.path()))
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file() && detect_language(e.path()).is_some())
        .map(|e| e.path().to_path_buf())
        .collect()
}

fn rel_path(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
}

fn is_ignored(path: &Path) -> bool {
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    matches!(
        name,
        ".git" | "target" | "node_modules" | "__pycache__" | ".venv" | "venv" | "bin"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use codegraph_ir::NodeKind;

    fn examples_go_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("examples/go-sample")
    }

    fn examples_py_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("examples/python-sample")
    }

    #[test]
    fn test_detect_language() {
        assert_eq!(detect_language(Path::new("a.go")), Some(Language::Go));
        assert_eq!(detect_language(Path::new("a.py")), Some(Language::Python));
        assert_eq!(detect_language(Path::new("a.rs")), Some(Language::Rust));
        assert_eq!(detect_language(Path::new("a.txt")), None);
    }

    #[test]
    fn test_parse_go_dir() {
        let dir = examples_go_dir();
        let graph = parse_dir(&dir).expect("parse_dir failed");

        assert_eq!(graph.node_count(), 8);
        assert_eq!(graph.edge_count(), 4);

        let validate = graph.find_by_qualified_name("auth.Validate");
        assert!(!validate.is_empty());
    }

    #[test]
    fn test_parse_python_dir() {
        let dir = examples_py_dir();
        if !dir.exists() {
            return;
        }
        let graph = parse_dir(&dir).expect("parse_dir failed");

        assert!(graph.node_count() > 0, "no nodes parsed from Python");
        assert!(graph.edge_count() > 0, "no edges parsed from Python");
        let mains: Vec<_> = graph
            .nodes
            .values()
            .filter(|n| n.kind == NodeKind::Main)
            .collect();
        assert!(!mains.is_empty(), "main() not found in Python sample");
    }

    #[test]
    fn test_list_source_files_includes_python() {
        let dir = examples_py_dir();
        if !dir.exists() {
            return;
        }
        let files = list_source_files(&dir);
        assert!(files.iter().any(|f| f.extension().unwrap() == "py"));
    }
        fn examples_rust_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("examples/rust-sample")
    }

    #[test]
    fn test_parse_rust_dir() {
        let dir = examples_rust_dir();
        if !dir.exists() {
            return;
        }
        let graph = parse_dir(&dir).expect("parse_dir failed");

        assert!(graph.node_count() > 0, "no nodes parsed from Rust");
        assert!(graph.edge_count() > 0, "no edges parsed from Rust");
    }
}