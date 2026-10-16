//Поиск dead code: недостижимого от entrypoints.

use codegraph_ir::Node;
use std::collections::HashSet;

use crate::queries::reachable_from;
use crate::CodeGraph;

pub struct DeadCodeResult {
    pub dead: Vec<Node>,
    pub total: usize,
}


pub fn find_dead_code(g: &CodeGraph, include_exported: bool) -> DeadCodeResult {
    let entrypoints: Vec<petgraph::graph::NodeIndex> = g
        .inner
        .node_indices()
        .filter(|&idx| {
            let kind = g.inner[idx].kind;
            kind.is_entrypoint()
        })
        .collect();

    let mut reachable: HashSet<petgraph::graph::NodeIndex> = HashSet::new();
    for &ep in &entrypoints {
        reachable.extend(reachable_from(g, ep));
    }

    let mut dead: Vec<Node> = g
        .inner
        .node_indices()
        .filter(|idx| !reachable.contains(idx))
        .filter(|idx| {
            let node = &g.inner[*idx];
            if node.kind.is_entrypoint() {
                return false;
            }
            if !include_exported && node.exported {
                return false;
            }
            true
        })
        .map(|idx| g.inner[idx].clone())
        .collect();

    dead.sort_by(|a, b| a.qualified_name.cmp(&b.qualified_name));
    let total = dead.len();

    DeadCodeResult { dead, total }
}