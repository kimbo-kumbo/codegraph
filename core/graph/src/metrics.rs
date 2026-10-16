//Метрики пакетов: coupling, instability, abstractness, distance.
use petgraph::visit::EdgeRef;
use codegraph_ir::NodeKind;
use std::collections::{HashMap, HashSet};

use crate::CodeGraph;

pub struct PackageMetrics {
    pub package: String,
    pub language: codegraph_ir::Language,
    pub fan_in: u32,
    pub fan_out: u32,
    pub instability: f64,
    pub abstractness: f64,
    pub distance: f64,
    pub nodes_count: u32,
    pub edges_count: u32,
}

pub fn get_metrics(g: &CodeGraph, package: &str) -> Vec<PackageMetrics> {
    let mut by_package: HashMap<String, Vec<petgraph::graph::NodeIndex>> = HashMap::new();
    for idx in g.inner.node_indices() {
        let node = &g.inner[idx];
        by_package
            .entry(node.package.clone())
            .or_default()
            .push(idx);
    }

    let mut result: Vec<PackageMetrics> = Vec::new();

    for (pkg, node_indices) in by_package {
        if !package.is_empty() && pkg != package {
            continue;
        }

        let nodes_set: HashSet<petgraph::graph::NodeIndex> = node_indices.iter().copied().collect();
        let mut fan_in: u32 = 0;
        let mut fan_out: u32 = 0;
        let mut edges_count: u32 = 0;

        for edge in g.inner.edge_references() {
            let src_in = nodes_set.contains(&edge.source());
            let dst_in = nodes_set.contains(&edge.target());
            edges_count += 1;

            if src_in && !dst_in {
                fan_out += 1;
            }
            if !src_in && dst_in {
                fan_in += 1;
            }
        }

        let language = node_indices
            .first()
            .map(|&idx| g.inner[idx].language)
            .unwrap_or(codegraph_ir::Language::Go);

        let ca = fan_in as f64;
        let ce = fan_out as f64;
        let instability = if ca + ce > 0.0 { ce / (ca + ce) } else { 0.0 };

        let total_types = node_indices
            .iter()
            .filter(|&&idx| {
                matches!(
                    g.inner[idx].kind,
                    NodeKind::Type | NodeKind::Interface
                )
            })
            .count() as f64;
        let abstract_types = node_indices
            .iter()
            .filter(|&&idx| g.inner[idx].kind == NodeKind::Interface)
            .count() as f64;
        let abstractness = if total_types > 0.0 {
            abstract_types / total_types
        } else {
            0.0
        };

        let distance = (abstractness + instability - 1.0).abs();

        result.push(PackageMetrics {
            package: pkg,
            language,
            fan_in,
            fan_out,
            instability,
            abstractness,
            distance,
            nodes_count: node_indices.len() as u32,
            edges_count,
        });
    }

    result.sort_by(|a, b| a.package.cmp(&b.package));
    result
}