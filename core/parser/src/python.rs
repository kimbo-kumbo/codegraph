//Python-специфичный экстрактор tree-sitter AST → IR.

use std::collections::HashMap;
use codegraph_ir::{Edge, EdgeKind, Graph, Language, Location, Node, NodeKind};
use tree_sitter::{Node as TsNode, Parser};
use crate::walker::Walker;
use crate::{ParseError, Result};

pub fn parse_declarations(
    source: &str,
    rel_path: &str,
) -> Result<(Graph, HashMap<String, String>)> {
    let tree = parse_tree(source)?;
    let root = tree.root_node();
    let walker = Walker::new(source, rel_path);

    let mut graph = Graph::new();
    let mut name_index: HashMap<String, String> = HashMap::new();
    let mut class_ranges: Vec<(usize, usize, String)> = Vec::new();
    walker.walk(root, &mut |ts_node, w| {
        if ts_node.kind() == "class_definition" {
            if let Some((node, name)) = extract_class(ts_node, w) {
                name_index.insert(name.clone(), node.id.clone());
                graph.add_node(node);
                let r = ts_node.byte_range();
                class_ranges.push((r.start, r.end, name));
            }
        }
    });

    walker.walk(root, &mut |ts_node, w| {
        if ts_node.kind() != "function_definition" {
            return;
        }

        let node_range = ts_node.byte_range();
        let class_name = class_ranges
            .iter()
            .find(|(start, end, _)| node_range.start >= *start && node_range.end <= *end)
            .map(|(_, _, name)| name.clone());

        let is_method = class_name.is_some();

        if let Some((node, short_name)) =
            extract_function(ts_node, w, is_method, class_name.as_deref())
        {
            name_index.entry(short_name).or_insert(node.id.clone());
            graph.add_node(node);
        }
    });

    Ok((graph, name_index))
}

pub fn parse_calls(
    source: &str,
    rel_path: &str,
    global_index: &HashMap<String, String>,
) -> Result<Graph> {
    let tree = parse_tree(source)?;
    let root = tree.root_node();
    let walker = Walker::new(source, rel_path);

    let mut graph = Graph::new();

    walker.walk(root, &mut |ts_node, w| {
        if ts_node.kind() == "call" {
            if let Some(edge) = extract_call(ts_node, w, global_index) {
                graph.add_edge(edge);
            }
        }
    });

    Ok(graph)
}

fn parse_tree(source: &str) -> Result<tree_sitter::Tree> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_python::LANGUAGE.into())
        .map_err(|e| ParseError::Parse(e.to_string()))?;

    parser
        .parse(source, None)
        .ok_or_else(|| ParseError::Parse("tree-sitter returned None".into()))
}

fn extract_class(ts_node: TsNode, walker: &Walker) -> Option<(Node, String)> {
    let name_node = ts_node.child_by_field_name("name")?;
    let name = walker.text(name_node).to_string();
    let (line, column) = walker.position(ts_node);
    let location = Location::new(walker.rel_path, line, column);

    let package = package_from_path(walker.rel_path);
    let qualified_name = format!("{}.{}", package, name);
    let exported = !name.starts_with('_');

    let node = Node::new(
        NodeKind::Type,
        &name,
        &qualified_name,
        &package,
        Language::Python,
        location,
    )
    .with_exported(exported);

    Some((node, name))
}

fn extract_function(
    ts_node: TsNode,
    walker: &Walker,
    is_method: bool,
    class_name: Option<&str>,
) -> Option<(Node, String)> {
    let name_node = ts_node.child_by_field_name("name")?;
    let name = walker.text(name_node).to_string();

    let kind = if is_method {
        NodeKind::Method
    } else if name == "main" {
        NodeKind::Main
    } else if name.starts_with("test_") || name.starts_with("Test") {
        NodeKind::Test
    } else {
        NodeKind::Function
    };

    let (line, column) = walker.position(ts_node);
    let location = Location::new(walker.rel_path, line, column);

    let package = package_from_path(walker.rel_path);
    let qualified_name = match class_name {
        Some(cls) => format!("{}.{}.{}", package, cls, name),
        None => format!("{}.{}", package, name),
    };
    let exported = !name.starts_with('_');
    let index_key = name.clone();

    let node = Node::new(
        kind,
        &name,
        &qualified_name,
        &package,
        Language::Python,
        location,
    )
    .with_exported(exported);

    Some((node, index_key))
}

fn extract_call(
    ts_node: TsNode,
    walker: &Walker,
    global_index: &HashMap<String, String>,
) -> Option<Edge> {
    let function = ts_node.child_by_field_name("function")?;

    let called_name = match function.kind() {
        "identifier" => walker.text(function).to_string(),
        "attribute" => {
            function
                .child_by_field_name("attribute")
                .map(|n| walker.text(n).to_string())?
        }
        _ => return None,
    };

    let target_id = global_index.get(&called_name)?.clone();
    let caller_node = find_enclosing_function(ts_node)?;
    let caller_name_node = caller_node.child_by_field_name("name")?;
    let caller_name = walker.text(caller_name_node);

    let caller_id = global_index.get(caller_name)?.clone();

    if caller_id == target_id {
        return None;
    }

    let (line, column) = walker.position(ts_node);
    Some(Edge::new(
        caller_id,
        target_id,
        EdgeKind::Calls,
        Location::new(walker.rel_path, line, column),
    ))
}

fn find_enclosing_function(node: TsNode) -> Option<TsNode> {
    let mut current = node.parent();
    while let Some(n) = current {
        if n.kind() == "function_definition" {
            return Some(n);
        }
        current = n.parent();
    }
    None
}

fn package_from_path(rel_path: &str) -> String {
    let path = std::path::Path::new(rel_path);
    path.parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .unwrap_or("__main__")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = r#"
import os


def validate(username: str) -> bool:
    return len(username) >= 3


class Service:
    def __init__(self, name: str):
        self.name = name

    def login(self, username: str) -> str:
        if validate(username):
            return "token-" + username
        return ""

    def logout(self, username: str) -> bool:
        return validate(username)


def main():
    svc = Service("x")
    svc.login("alice")
    svc.logout("alice")


if __name__ == "__main__":
    main()
"#;

    #[test]
    fn test_parse_declarations() {
        let (graph, index) = parse_declarations(SRC, "auth/auth.py").unwrap();
        assert_eq!(graph.node_count(), 6, "got {:?}", graph.nodes.keys().collect::<Vec<_>>());
        assert!(index.contains_key("validate"));
        assert!(index.contains_key("Service"));
        assert!(index.contains_key("login"));
        assert!(index.contains_key("logout"));
        assert!(index.contains_key("main"));
    }

    #[test]
    fn test_parse_calls() {
        let (_, index) = parse_declarations(SRC, "auth/auth.py").unwrap();
        let calls = parse_calls(SRC, "auth/auth.py", &index).unwrap();
        assert_eq!(calls.edge_count(), 5, "edges: {:?}", calls.edges);
    }

    #[test]
    fn test_class_kind_is_type() {
        let (graph, _) = parse_declarations(SRC, "auth/auth.py").unwrap();
        let cls = graph.nodes.values().find(|n| n.name == "Service").unwrap();
        assert_eq!(cls.kind, NodeKind::Type);
    }

    #[test]
    fn test_method_kind() {
        let (graph, _) = parse_declarations(SRC, "auth/auth.py").unwrap();
        let m = graph.nodes.values().find(|n| n.name == "login").unwrap();
        assert_eq!(m.kind, NodeKind::Method);
    }
}