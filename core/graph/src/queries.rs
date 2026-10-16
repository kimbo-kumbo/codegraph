//! Запросы к графу: кто вызывает, что сломается.

use codegraph_ir::{Node, NodeKind};
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use std::collections::{HashMap, HashSet, VecDeque};
use crate::CodeGraph;

pub struct CallersResult {
    pub symbol: Node,
    pub callers: Vec<Node>,
    pub total: usize,
}

pub fn find_callers(
    g: &CodeGraph,
    qualified_name: &str,
    transitive: bool,
    max_depth: u32,
) -> Option<CallersResult> {
    let target_idx = find_node_by_qualified_name(g, qualified_name)?;
    let symbol = g.inner[target_idx].clone();

    let mut callers: Vec<Node> = Vec::new();

    if !transitive {
        for caller_idx in g.incoming_callers(target_idx) {
            callers.push(g.inner[caller_idx].clone());
        }
    } else {
        let mut visited: HashSet<NodeIndex> = HashSet::new();
        let mut queue: VecDeque<(NodeIndex, u32)> = VecDeque::new();

        for caller_idx in g.incoming_callers(target_idx) {
            if visited.insert(caller_idx) {
                queue.push_back((caller_idx, 1));
            }
        }

        while let Some((idx, depth)) = queue.pop_front() {
            callers.push(g.inner[idx].clone());

            if max_depth > 0 && depth >= max_depth {
                continue;
            }

            for caller_idx in g.incoming_callers(idx) {
                if visited.insert(caller_idx) {
                    queue.push_back((caller_idx, depth + 1));
                }
            }
        }
    }

    let total = callers.len();
    Some(CallersResult {
        symbol,
        callers,
        total,
    })
}

pub struct ImpactResult {
    pub symbol: Node,
    pub affected: Vec<Node>,
    pub affected_packages: Vec<String>,
    pub affected_routes: Vec<Node>,
    pub total_functions: usize,
    pub total_packages: usize,
}

pub fn impact_of_change(
    g: &CodeGraph,
    qualified_name: &str,
    include_tests: bool,
) -> Option<ImpactResult> {
    let callers = find_callers(g, qualified_name, true, 0)?;

    let affected: Vec<Node> = if include_tests {
        callers.callers.clone()
    } else {
        callers
            .callers
            .iter()
            .filter(|n| n.kind != NodeKind::Test)
            .cloned()
            .collect()
    };

    let mut pkgs: HashSet<String> = HashSet::new();
    for n in &affected {
        pkgs.insert(n.package.clone());
    }
    let mut affected_packages: Vec<String> = pkgs.into_iter().collect();
    affected_packages.sort();

    let affected_routes: Vec<Node> = affected
        .iter()
        .filter(|n| n.kind == NodeKind::Route)
        .cloned()
        .collect();

    let total_functions = affected.len();
    let total_packages = affected_packages.len();

    Some(ImpactResult {
        symbol: callers.symbol,
        affected,
        affected_packages,
        affected_routes,
        total_functions,
        total_packages,
    })
}

pub fn find_node_by_qualified_name(g: &CodeGraph, qn: &str) -> Option<NodeIndex> {
    if let Some(&idx) = g.qualified_to_index.get(qn) {
        return Some(idx);
    }

    for node in g.inner.node_weights() {
        if node.name == qn || node.qualified_name.ends_with(&format!(".{}", qn)) {
            return g.node_by_id(&node.id);
        }
    }

    None
}

pub fn build_adjacency(g: &CodeGraph) -> HashMap<NodeIndex, Vec<NodeIndex>> {
    let mut adj: HashMap<NodeIndex, Vec<NodeIndex>> = HashMap::new();
    for edge in g.inner.edge_references() {
        adj.entry(edge.source()).or_default().push(edge.target());
    }
    adj
}

pub fn reachable_from(g: &CodeGraph, start: NodeIndex) -> HashSet<NodeIndex> {
    let mut visited: HashSet<NodeIndex> = HashSet::new();
    let mut queue: VecDeque<NodeIndex> = VecDeque::new();
    queue.push_back(start);
    visited.insert(start);

    while let Some(idx) = queue.pop_front() {
        for next in g
            .inner
            .neighbors_directed(idx, Direction::Outgoing)
        {
            if visited.insert(next) {
                queue.push_back(next);
            }
        }
    }
    visited
}