//Go-специфичный экстрактор tree-sitter AST → IR.

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

    walker.walk(root, &mut |ts_node, w| {
        if let Some((node, short_name)) = extract_declaration(ts_node, w) {
            name_index.insert(short_name, node.id.clone());
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
        if ts_node.kind() == "call_expression" {
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
        .set_language(&tree_sitter_go::LANGUAGE.into())
        .map_err(|e| ParseError::Parse(e.to_string()))?;

    parser
        .parse(source, None)
        .ok_or_else(|| ParseError::Parse("tree-sitter returned None".into()))
}

fn extract_declaration(ts_node: TsNode, walker: &Walker) -> Option<(Node, String)> {
    match ts_node.kind() {
        "function_declaration" => extract_function(ts_node, walker),
        "method_declaration" => extract_method(ts_node, walker),
        "type_declaration" => extract_type(ts_node, walker),
        _ => None,
    }
}

fn extract_function(ts_node: TsNode, walker: &Walker) -> Option<(Node, String)> {
    let name_node = walker.child_of_kind(ts_node, "identifier")?;
    let name = walker.text(name_node).to_string();
    let (line, column) = walker.position(ts_node);
    let location = Location::new(walker.rel_path, line, column);
    let kind = if name == "main" {
        NodeKind::Main
    } else if name.starts_with("Test") || name.starts_with("Benchmark") {
        NodeKind::Test
    } else {
        NodeKind::Function
    };

    let package = package_from_path(walker.rel_path);
    let qualified_name = format!("{}.{}", package, name);
    let exported = name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false);

    let node = Node::new(kind, &name, &qualified_name, &package, Language::Go, location)
        .with_exported(exported);

    Some((node, name))
}

fn extract_method(ts_node: TsNode, walker: &Walker) -> Option<(Node, String)> {
    let name_node = walker.child_of_kind(ts_node, "field_identifier")?;
    let name = walker.text(name_node).to_string();
    let (line, column) = walker.position(ts_node);
    let location = Location::new(walker.rel_path, line, column);

    let package = package_from_path(walker.rel_path);
    let qualified_name = format!("{}.{}", package, name);
    let exported = name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false);

    let node = Node::new(NodeKind::Method, &name, &qualified_name, &package, Language::Go, location)
        .with_exported(exported);

    Some((node, name))
}

fn extract_type(ts_node: TsNode, walker: &Walker) -> Option<(Node, String)> {
    let spec = walker.child_of_kind(ts_node, "type_spec")?;
    let name_node = walker.child_of_kind(spec, "type_identifier")?;
    let name = walker.text(name_node).to_string();

    let kind = if walker.has_child_of_kind(spec, "interface_type") {
        NodeKind::Interface
    } else {
        NodeKind::Type
    };

    let (line, column) = walker.position(ts_node);
    let location = Location::new(walker.rel_path, line, column);
    let package = package_from_path(walker.rel_path);
    let qualified_name = format!("{}.{}", package, name);
    let exported = name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false);

    let node = Node::new(kind, &name, &qualified_name, &package, Language::Go, location)
        .with_exported(exported);

    Some((node, name))
}

fn extract_call(
    ts_node: TsNode,
    walker: &Walker,
    global_index: &HashMap<String, String>,
) -> Option<Edge> {
    let function = ts_node.child_by_field_name("function")?;

    let called_name = match function.kind() {
        "identifier" => walker.text(function).to_string(),
        "selector_expression" => {
            let field = function.child_by_field_name("field")?;
            walker.text(field).to_string()
        }
        _ => return None,
    };

    let target_id = global_index.get(&called_name)?.clone();
    let caller_node = find_enclosing_function(ts_node)?;
    let caller_name_node = walker
        .child_of_kind(caller_node, "identifier")
        .or_else(|| walker.child_of_kind(caller_node, "field_identifier"))?;
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
        if matches!(n.kind(), "function_declaration" | "method_declaration") {
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
        .unwrap_or("main")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = r#"
package auth

import "errors"

func Validate(username string) error {
    if len(username) < 3 {
        return errors.New("short")
    }
    return nil
}

func Login(username string) (string, error) {
    if err := Validate(username); err != nil {
        return "", err
    }
    return "token-" + username, nil
}

func Logout(username string) error {
    return Validate(username)
}
"#;

    #[test]
    fn test_parse_declarations() {
        let (graph, index) = parse_declarations(SRC, "internal/auth/auth.go").unwrap();
        assert_eq!(graph.node_count(), 3);
        assert!(index.contains_key("Validate"));
        assert!(index.contains_key("Login"));
        assert!(index.contains_key("Logout"));
    }

    #[test]
    fn test_parse_calls_with_global_index() {
        let (_, index) = parse_declarations(SRC, "internal/auth/auth.go").unwrap();
        let calls = parse_calls(SRC, "internal/auth/auth.go", &index).unwrap();
        assert_eq!(calls.edge_count(), 2, "expected 2 CALLS edges");
    }

    #[test]
    fn test_extract_main_is_main_kind() {
        let src = "package main\n\nfunc main() {}\n";
        let (graph, _) = parse_declarations(src, "main.go").unwrap();
        let main_node = graph.nodes.values().find(|n| n.name == "main").unwrap();
        assert_eq!(main_node.kind, NodeKind::Main);
    }

    #[test]
    fn test_extract_test_kind() {
        let src = "package foo\n\nfunc TestBar() {}\n";
        let (graph, _) = parse_declarations(src, "foo_test.go").unwrap();
        let test_node = graph.nodes.values().find(|n| n.name == "TestBar").unwrap();
        assert_eq!(test_node.kind, NodeKind::Test);
    }
}