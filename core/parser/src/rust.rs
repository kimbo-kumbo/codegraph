//Rust-специфичный экстрактор tree-sitter AST → IR.

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
    let mut impl_ranges: Vec<(usize, usize, String)> = Vec::new();

    walker.walk(root, &mut |ts_node, w| {
        match ts_node.kind() {
            "struct_item" | "enum_item" | "union_item" => {
                if let Some((node, name)) = extract_type(ts_node, w, NodeKind::Type) {
                    name_index.insert(name.clone(), node.id.clone());
                    graph.add_node(node);
                }
            }
            "trait_item" => {
                if let Some((node, name)) = extract_type(ts_node, w, NodeKind::Interface) {
                    name_index.insert(name.clone(), node.id.clone());
                    graph.add_node(node);
                }
            }
            "impl_item" => {
                if let Some(type_name) = extract_impl_target(ts_node, w) {
                    let range = ts_node.byte_range();
                    impl_ranges.push((range.start, range.end, type_name));
                }
            }
            _ => {}
        }
    });

    walker.walk(root, &mut |ts_node, w| {
        if ts_node.kind() != "function_item" {
            return;
        }
        let node_range = ts_node.byte_range();
        let impl_type = impl_ranges
            .iter()
            .find(|(start, end, _)| node_range.start >= *start && node_range.end <= *end)
            .map(|(_, _, name)| name.clone());

        let is_method = impl_type.is_some();

        if let Some((node, short_name)) =
            extract_function(ts_node, w, is_method, impl_type.as_deref())
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
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .map_err(|e| ParseError::Parse(e.to_string()))?;

    parser
        .parse(source, None)
        .ok_or_else(|| ParseError::Parse("tree-sitter returned None".into()))
}

fn extract_type(ts_node: TsNode, walker: &Walker, kind: NodeKind) -> Option<(Node, String)> {
    let name_node = ts_node.child_by_field_name("name")?;
    let name = walker.text(name_node).to_string();
    let (line, column) = walker.position(ts_node);
    let location = Location::new(walker.rel_path, line, column);
    let package = package_from_path(walker.rel_path);
    let qualified_name = format!("{}.{}", package, name);
    let exported = is_exported(ts_node, walker);

    let node = Node::new(kind, &name, &qualified_name, &package, Language::Rust, location)
        .with_exported(exported);

    Some((node, name))
}

fn extract_impl_target(ts_node: TsNode, walker: &Walker) -> Option<String> {
    let type_node = ts_node.child_by_field_name("type")?;
    Some(walker.text(type_node).to_string())
}

fn extract_function(
    ts_node: TsNode,
    walker: &Walker,
    is_method: bool,
    impl_type: Option<&str>,
) -> Option<(Node, String)> {
    let name_node = ts_node.child_by_field_name("name")?;
    let name = walker.text(name_node).to_string();
    let is_test = has_test_attribute(ts_node, walker);

    let kind = if is_test {
        NodeKind::Test
    } else if name == "main" {
        NodeKind::Main
    } else if is_method {
        NodeKind::Method
    } else {
        NodeKind::Function
    };

    let (line, column) = walker.position(ts_node);
    let location = Location::new(walker.rel_path, line, column);

    let package = package_from_path(walker.rel_path);
    let qualified_name = match impl_type {
        Some(t) => format!("{}.{}.{}", package, t, name),
        None => format!("{}.{}", package, name),
    };
    let exported = is_exported(ts_node, walker);
    let index_key = name.clone();
    let node = Node::new(kind, &name, &qualified_name, &package, Language::Rust, location)
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
        "field_expression" => {
            function
                .child_by_field_name("field")
                .map(|n| walker.text(n).to_string())?
        }
        "scoped_identifier" => {
            let name = function.child_by_field_name("name")?;
            walker.text(name).to_string()
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
        if n.kind() == "function_item" {
            return Some(n);
        }
        current = n.parent();
    }
    None
}

fn is_exported(node: TsNode, walker: &Walker) -> bool {
    let mut cursor = node.walk();
    let result = node.children(&mut cursor).any(|c| {
        c.kind() == "visibility_modifier" && walker.text(c).starts_with("pub")
    });
    result
}

fn has_test_attribute(node: TsNode, walker: &Walker) -> bool {
    let mut prev = node.prev_sibling();
    while let Some(p) = prev {
        if p.kind() == "attribute_item" {
            let text = walker.text(p);
            if text.contains("test") {
                return true;
            }
        } else if p.kind() != "line_comment" && p.kind() != "block_comment" {
            break;
        }
        prev = p.prev_sibling();
    }
    false
}

fn package_from_path(rel_path: &str) -> String {
    let path = std::path::Path::new(rel_path);
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("main")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = r#"
pub fn validate(username: &str) -> bool {
    username.len() >= 3
}

pub struct Service;

impl Service {
    pub fn login(&self, username: &str) -> String {
        if validate(username) {
            return format!("token-{}", username);
        }
        String::new()
    }

    pub fn logout(&self, username: &str) -> bool {
        validate(username)
    }
}

fn hash_password(password: &str) -> String {
    format!("hashed:{}", password)
}

fn main() {
    let svc = Service;
    svc.login("alice");
    svc.logout("alice");
}

#[test]
fn test_validate() {
    assert!(validate("alice"));
}
"#;

    #[test]
    fn test_parse_declarations() {
        let (graph, index) = parse_declarations(SRC, "auth/auth.rs").unwrap();
        assert_eq!(
            graph.node_count(),
            7,
            "got {:?}",
            graph.nodes.keys().collect::<Vec<_>>()
        );

        assert!(index.contains_key("validate"));
        assert!(index.contains_key("Service"));
        assert!(index.contains_key("login"));
        assert!(index.contains_key("logout"));
        assert!(index.contains_key("main"));
    }

    #[test]
    fn test_parse_calls() {
        let (_, index) = parse_declarations(SRC, "auth/auth.rs").unwrap();
        let calls = parse_calls(SRC, "auth/auth.rs", &index).unwrap();
        assert_eq!(calls.edge_count(), 4, "edges: {:?}", calls.edges);
    }

    #[test]
    fn test_trait_is_interface() {
        let src = "pub trait Repo { fn find(&self, id: u32); }";
        let (graph, _) = parse_declarations(src, "repo.rs").unwrap();
        let t = graph.nodes.values().find(|n| n.name == "Repo").unwrap();
        assert_eq!(t.kind, NodeKind::Interface);
    }

    #[test]
    fn test_main_is_main_kind() {
        let src = "fn main() {}";
        let (graph, _) = parse_declarations(src, "main.rs").unwrap();
        let m = graph.nodes.values().find(|n| n.name == "main").unwrap();
        assert_eq!(m.kind, NodeKind::Main);
    }

    #[test]
    fn test_method_kind() {
        let (graph, _) = parse_declarations(SRC, "auth/auth.rs").unwrap();
        let m = graph.nodes.values().find(|n| n.name == "login").unwrap();
        assert_eq!(m.kind, NodeKind::Method);
    }

    #[test]
    fn test_test_kind() {
        let (graph, _) = parse_declarations(SRC, "auth/auth.rs").unwrap();
        let t = graph
            .nodes
            .values()
            .find(|n| n.name == "test_validate")
            .unwrap();
        assert_eq!(t.kind, NodeKind::Test);
    }

        #[test]
    fn test_debug_kinds() {
        let src = "pub fn main() { let x = 1; }";
        let tree = parse_tree(src).unwrap();
        let root = tree.root_node();
        eprintln!("[debug] root kind = {}", root.kind());

        fn walk(n: TsNode, depth: usize) {
            let indent = "  ".repeat(depth);
            eprintln!("[debug] {}{}", indent, n.kind());
            let mut cursor = n.walk();
            for child in n.children(&mut cursor) {
                walk(child, depth + 1);
            }
        }
        walk(root, 0);

        let (graph, _) = parse_declarations(src, "test.rs").unwrap();
        eprintln!("[debug] nodes = {}", graph.node_count());
    }
}