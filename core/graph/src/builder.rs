//Построение petgraph из IR-графа.
use codegraph_ir::{EdgeKind, Graph as IrGraph};
use petgraph::graph::DiGraph;
use std::collections::HashMap;

use crate::CodeGraph;
pub fn build(ir: &IrGraph) -> CodeGraph {
    let mut inner: DiGraph<codegraph_ir::Node, EdgeKind> = DiGraph::new();
    let mut id_to_index: HashMap<String, petgraph::graph::NodeIndex> = HashMap::new();
    let mut qualified_to_index: HashMap<String, petgraph::graph::NodeIndex> = HashMap::new();

    for node in ir.nodes.values() {
        let idx = inner.add_node(node.clone());
        id_to_index.insert(node.id.clone(), idx);
        qualified_to_index
            .entry(node.qualified_name.clone())
            .or_insert(idx);
    }

    for edge in &ir.edges {
        if let (Some(&from), Some(&to)) = (
            id_to_index.get(&edge.from),
            id_to_index.get(&edge.to),
        ) {
            inner.add_edge(from, to, edge.kind);
        }
    }

    CodeGraph {
        inner,
        id_to_index,
        qualified_to_index,
    }
}