//! CodeGraph Graph — граф кода с запросами.
pub mod builder;
pub mod dead_code;
pub mod metrics;
pub mod queries;

use codegraph_ir::Graph as IrGraph;
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;

pub struct CodeGraph {
    pub inner: DiGraph<codegraph_ir::Node, codegraph_ir::EdgeKind>,
    pub id_to_index: HashMap<String, NodeIndex>,
    pub qualified_to_index: HashMap<String, NodeIndex>,
}

impl CodeGraph {
    pub fn from_ir(ir: &IrGraph) -> Self {
        builder::build(ir)
    }

    pub fn node_by_id(&self, id: &str) -> Option<NodeIndex> {
        self.id_to_index.get(id).copied()
    }

    pub fn nodes_by_qualified_name(&self, qn: &str) -> Vec<NodeIndex> {
        let mut result = Vec::new();
        for idx in self.inner.node_indices() {
            let node = &self.inner[idx];
            if node.qualified_name == qn {
                result.push(idx);
            }
        }
        result
    }

    pub fn incoming_callers(&self, target: NodeIndex) -> Vec<NodeIndex> {
        self.inner
            .neighbors_directed(target, petgraph::Direction::Incoming)
            .collect()
    }

    pub fn outgoing_callees(&self, source: NodeIndex) -> Vec<NodeIndex> {
        self.inner
            .neighbors_directed(source, petgraph::Direction::Outgoing)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn examples_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("examples/go-sample")
    }

    fn build_examples_graph() -> CodeGraph {
        let dir = examples_dir();
        let ir = codegraph_parser::parse_dir(&dir).expect("parse_dir failed");
        CodeGraph::from_ir(&ir)
    }

    #[test]
    fn test_build_code_graph() {
        let g = build_examples_graph();
        assert_eq!(g.inner.node_count(), 8);
        assert_eq!(g.inner.edge_count(), 4);
    }

    #[test]
    fn test_find_callers_direct() {
        let g = build_examples_graph();
        let result = queries::find_callers(&g, "auth.Validate", false, 0)
            .expect("auth.Validate not found");
        assert_eq!(result.total, 2);

        let names: Vec<String> = result
            .callers
            .iter()
            .map(|n| n.qualified_name.clone())
            .collect();
        assert!(names.iter().any(|n| n == "auth.Login"));
        assert!(names.iter().any(|n| n == "auth.Logout"));
    }

    #[test]
    fn test_find_callers_transitive() {
        let g = build_examples_graph();
        let result = queries::find_callers(&g, "auth.Validate", true, 0)
            .expect("auth.Validate not found");
        assert_eq!(result.total, 3);

        let names: Vec<String> = result
            .callers
            .iter()
            .map(|n| n.qualified_name.clone())
            .collect();
        assert!(names.iter().any(|n| n == "main.main"));
        assert!(names.iter().any(|n| n == "auth.Login"));
        assert!(names.iter().any(|n| n == "auth.Logout"));
    }

    #[test]
    fn test_find_callers_by_short_name() {
        let g = build_examples_graph();
        let result = queries::find_callers(&g, "Validate", false, 0)
            .expect("Validate not found");
        assert_eq!(result.total, 2);
    }

    #[test]
    fn test_impact_of_change() {
        let g = build_examples_graph();
        let result = queries::impact_of_change(&g, "auth.Validate", false)
            .expect("impact failed");

        assert_eq!(result.total_functions, 3);
        assert!(result.affected_packages.contains(&"auth".to_string()));
        assert!(result.affected_packages.contains(&"main".to_string()));
    }

    #[test]
    fn test_dead_code_without_exported() {
        let g = build_examples_graph();
        let result = dead_code::find_dead_code(&g, false);
        assert_eq!(result.total, 0);
    }

    #[test]
    fn test_dead_code_with_exported() {
        let g = build_examples_graph();
        let result = dead_code::find_dead_code(&g, true);
        assert_eq!(result.total, 4);

        let names: Vec<String> = result.dead.iter().map(|n| n.name.clone()).collect();
        assert!(
            names.contains(&"Logout".to_string()),
            "Logout not in dead: {:?}",
            names
        );
        assert!(names.contains(&"HashPassword".to_string()));
        assert!(names.contains(&"Query".to_string()));
    }

    #[test]
    fn test_metrics_for_auth() {
        let g = build_examples_graph();
        let metrics = metrics::get_metrics(&g, "auth");
        assert_eq!(metrics.len(), 1);

        let m = &metrics[0];
        assert_eq!(m.package, "auth");
        assert_eq!(m.nodes_count, 4);
        assert!(m.fan_out > 0);
    }

    #[test]
    fn test_metrics_for_all() {
        let g = build_examples_graph();
        let metrics = metrics::get_metrics(&g, "");
        assert_eq!(metrics.len(), 4);
    }
}