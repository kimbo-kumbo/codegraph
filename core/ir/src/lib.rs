//CodeGraph IR — промежуточное представление графа кода.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Go,
    Python,
    Rust,
}

impl Language {
    pub fn as_str(&self) -> &'static str {
        match self {
            Language::Go => "go",
            Language::Python => "python",
            Language::Rust => "rust",
        }
    }

    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "go" => Some(Language::Go),
            "py" => Some(Language::Python),
            "rs" => Some(Language::Rust),
            _ => None,
        }
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeKind {
    Function,
    Method,
    Type,
    Interface,
    Package,
    Route,
    Test,
    Main,
}

impl NodeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            NodeKind::Function => "function",
            NodeKind::Method => "method",
            NodeKind::Type => "type",
            NodeKind::Interface => "interface",
            NodeKind::Package => "package",
            NodeKind::Route => "route",
            NodeKind::Test => "test",
            NodeKind::Main => "main",
        }
    }

    pub fn is_entrypoint(&self) -> bool {
        matches!(self, NodeKind::Main | NodeKind::Route | NodeKind::Test)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EdgeKind {
    Calls,
    Imports,
    Implements,
    References,
}

impl EdgeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            EdgeKind::Calls => "calls",
            EdgeKind::Imports => "imports",
            EdgeKind::Implements => "implements",
            EdgeKind::References => "references",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Location {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

impl Location {
    pub fn new(file: impl Into<String>, line: u32, column: u32) -> Self {
        Self {
            file: file.into(),
            line,
            column,
        }
    }

    pub fn short(&self) -> String {
        format!("{}:{}:{}", self.file, self.line, self.column)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub kind: NodeKind,
    pub name: String,
    pub qualified_name: String,
    pub package: String,
    pub language: Language,
    pub location: Location,
    pub exported: bool,
    #[serde(default)]
    pub fan_in: u32,
    #[serde(default)]
    pub fan_out: u32,
}

impl Node {
    pub fn new(
        kind: NodeKind,
        name: impl Into<String>,
        qualified_name: impl Into<String>,
        package: impl Into<String>,
        language: Language,
        location: Location,
    ) -> Self {
        let name = name.into();
        let qualified_name = qualified_name.into();
        let package = package.into();
        let id = format!("{}:{}:{}", language.as_str(), package, name);

        Self {
            id,
            kind,
            name,
            qualified_name,
            package,
            language,
            location,
            exported: false,
            fan_in: 0,
            fan_out: 0,
        }
    }

    pub fn with_exported(mut self, exported: bool) -> Self {
        self.exported = exported;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub kind: EdgeKind,
    pub location: Location,
}

impl Edge {
    pub fn new(
        from: impl Into<String>,
        to: impl Into<String>,
        kind: EdgeKind,
        location: Location,
    ) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            kind,
            location,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Graph {
    pub nodes: HashMap<String, Node>,
    pub edges: Vec<Edge>,
    #[serde(skip)]
    pub outgoing: HashMap<String, Vec<String>>,
    #[serde(skip)]
    pub incoming: HashMap<String, Vec<String>>,
}

impl Graph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: Node) {
        self.nodes.insert(node.id.clone(), node);
    }

    pub fn add_edge(&mut self, edge: Edge) {
        self.outgoing
            .entry(edge.from.clone())
            .or_default()
            .push(edge.to.clone());
        self.incoming
            .entry(edge.to.clone())
            .or_default()
            .push(edge.from.clone());
        self.edges.push(edge);
    }

    pub fn get_node(&self, id: &str) -> Option<&Node> {
        self.nodes.get(id)
    }

    pub fn find_by_qualified_name(&self, qn: &str) -> Vec<&Node> {
        self.nodes
            .values()
            .filter(|n| n.qualified_name == qn)
            .collect()
    }

    pub fn compute_fan_metrics(&mut self) {
        for node in self.nodes.values_mut() {
            node.fan_in = self
                .incoming
                .get(&node.id)
                .map(|v| v.len() as u32)
                .unwrap_or(0);
            node.fan_out = self
                .outgoing
                .get(&node.id)
                .map(|v| v.len() as u32)
                .unwrap_or(0);
        }
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn packages(&self) -> Vec<String> {
        let mut pkgs: Vec<String> = self
            .nodes
            .values()
            .map(|n| n.package.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        pkgs.sort();
        pkgs
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Info => "info",
            Severity::Warning => "warning",
            Severity::Error => "error",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_go_node(name: &str, qualified: &str, package: &str) -> Node {
        Node::new(
            NodeKind::Function,
            name,
            qualified,
            package,
            Language::Go,
            Location::new("internal/auth/auth.go", 15, 1),
        )
    }

    #[test]
    fn test_language_as_str() {
        assert_eq!(Language::Go.as_str(), "go");
        assert_eq!(Language::Python.as_str(), "python");
        assert_eq!(Language::Rust.as_str(), "rust");
    }

    #[test]
    fn test_language_from_extension() {
        assert_eq!(Language::from_extension("go"), Some(Language::Go));
        assert_eq!(Language::from_extension("py"), Some(Language::Python));
        assert_eq!(Language::from_extension("rs"), Some(Language::Rust));
        assert_eq!(Language::from_extension("txt"), None);
    }

    #[test]
    fn test_node_autogenerated_id() {
        let node = make_go_node("Validate", "auth.Validate", "auth");
        assert_eq!(node.id, "go:auth:Validate");
        assert_eq!(node.name, "Validate");
        assert_eq!(node.qualified_name, "auth.Validate");
        assert_eq!(node.package, "auth");
        assert_eq!(node.language, Language::Go);
    }

    #[test]
    fn test_node_kind_is_entrypoint() {
        assert!(NodeKind::Main.is_entrypoint());
        assert!(NodeKind::Route.is_entrypoint());
        assert!(NodeKind::Test.is_entrypoint());
        assert!(!NodeKind::Function.is_entrypoint());
        assert!(!NodeKind::Package.is_entrypoint());
    }

    #[test]
    fn test_graph_add_node_and_edge() {
        let mut g = Graph::new();

        let validate = make_go_node("Validate", "auth.Validate", "auth");
        let login = make_go_node("Login", "auth.Login", "auth");

        g.add_node(validate.clone());
        g.add_node(login.clone());

        assert_eq!(g.node_count(), 2);

        g.add_edge(Edge::new(
            login.id.clone(),
            validate.id.clone(),
            EdgeKind::Calls,
            Location::new("internal/auth/auth.go", 30, 5),
        ));

        assert_eq!(g.edge_count(), 1);
        assert_eq!(g.outgoing.get(&login.id).unwrap().len(), 1);
        assert_eq!(g.incoming.get(&validate.id).unwrap().len(), 1);
    }

    #[test]
    fn test_graph_compute_fan_metrics() {
        let mut g = Graph::new();

        let a = make_go_node("A", "pkg.A", "pkg");
        let b = make_go_node("B", "pkg.B", "pkg");
        let c = make_go_node("C", "pkg.C", "pkg");

        g.add_node(a.clone());
        g.add_node(b.clone());
        g.add_node(c.clone());

        // A → B, A → C
        g.add_edge(Edge::new(&a.id, &b.id, EdgeKind::Calls, Location::default()));
        g.add_edge(Edge::new(&a.id, &c.id, EdgeKind::Calls, Location::default()));
        // B → C
        g.add_edge(Edge::new(&b.id, &c.id, EdgeKind::Calls, Location::default()));

        g.compute_fan_metrics();

        let a_metrics = g.get_node(&a.id).unwrap();
        assert_eq!(a_metrics.fan_out, 2);
        assert_eq!(a_metrics.fan_in, 0);

        let c_metrics = g.get_node(&c.id).unwrap();
        assert_eq!(c_metrics.fan_in, 2);
        assert_eq!(c_metrics.fan_out, 0);
    }

    #[test]
    fn test_graph_find_by_qualified_name() {
        let mut g = Graph::new();
        g.add_node(make_go_node("Validate", "auth.Validate", "auth"));
        g.add_node(make_go_node("Validate", "billing.Validate", "billing"));

        let found = g.find_by_qualified_name("auth.Validate");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].package, "auth");
    }

    #[test]
    fn test_graph_packages() {
        let mut g = Graph::new();
        g.add_node(make_go_node("Validate", "auth.Validate", "auth"));
        g.add_node(make_go_node("Login", "auth.Login", "auth"));
        g.add_node(make_go_node("Query", "db.Query", "db"));

        let pkgs = g.packages();
        assert_eq!(pkgs, vec!["auth".to_string(), "db".to_string()]);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let mut g = Graph::new();
        let a = make_go_node("Validate", "auth.Validate", "auth");
        let b = make_go_node("Login", "auth.Login", "auth");
        g.add_node(a.clone());
        g.add_node(b.clone());
        g.add_edge(Edge::new(&b.id, &a.id, EdgeKind::Calls, Location::default()));

        let json = serde_json::to_string(&g).unwrap();
        let g2: Graph = serde_json::from_str(&json).unwrap();

        assert_eq!(g.node_count(), g2.node_count());
        assert_eq!(g.edge_count(), g2.edge_count());
    }
}