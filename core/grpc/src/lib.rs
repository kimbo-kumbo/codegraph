//CodeGraph gRPC server — реализация сервиса из `codegraph.proto`.

use std::sync::Arc;

use codegraph_graph::CodeGraph;
use codegraph_ir::{EdgeKind, Language, NodeKind};
use tokio::sync::RwLock;
use tonic::{Request, Response, Status};

pub mod pb {
    tonic::include_proto!("codegraph.v1");
}

use pb::code_graph_server::CodeGraph as CodeGraphTrait;
use pb::*;

#[derive(Default)]
pub struct GraphState {
    pub graph: Option<CodeGraph>,
    pub project_path: Option<std::path::PathBuf>,
}
pub type SharedState = Arc<RwLock<GraphState>>;

pub struct CodeGraphService {
    state: SharedState,
}

impl CodeGraphService {
    pub fn new(state: SharedState) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl CodeGraphTrait for CodeGraphService {
    async fn index(
        &self,
        request: Request<IndexRequest>,
    ) -> Result<Response<IndexResponse>, Status> {
        let req = request.into_inner();
        let project_path = req.project_path.clone();

        if project_path.is_empty() {
            return Err(Status::invalid_argument("project_path is required"));
        }

        let start = std::time::Instant::now();
        let path = std::path::PathBuf::from(&project_path);

        let ir_graph = codegraph_parser::parse_dir(&path)
            .map_err(|e| Status::internal(format!("parse error: {}", e)))?;

        let files_parsed = count_source_files(&path);
        let nodes_created = ir_graph.node_count() as u32;
        let edges_created = ir_graph.edge_count() as u32;

        let code_graph = CodeGraph::from_ir(&ir_graph);

        {
            let mut state = self.state.write().await;
            state.graph = Some(code_graph);
            state.project_path = Some(path.clone());
        }

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(Response::new(IndexResponse {
            files_parsed,
            nodes_created,
            edges_created,
            duration_ms,
            errors: vec![],
        }))
    }

    async fn find_callers(
        &self,
        request: Request<FindCallersRequest>,
    ) -> Result<Response<FindCallersResponse>, Status> {
        let req = request.into_inner();
        let state = self.state.read().await;
        let graph = state
            .graph
            .as_ref()
            .ok_or_else(|| Status::failed_precondition("call Index first"))?;

        let result = codegraph_graph::queries::find_callers(
            graph,
            &req.symbol,
            req.transitive,
            req.max_depth,
        )
        .ok_or_else(|| Status::not_found(format!("symbol not found: {}", req.symbol)))?;

        let response = FindCallersResponse {
            symbol: Some(node_to_pb(&result.symbol)),
            callers: result.callers.iter().map(node_to_pb).collect(),
            total: result.total as u32,
        };

        Ok(Response::new(response))
    }

    async fn impact_of_change(
        &self,
        request: Request<ImpactOfChangeRequest>,
    ) -> Result<Response<ImpactOfChangeResponse>, Status> {
        let req = request.into_inner();
        let state = self.state.read().await;
        let graph = state
            .graph
            .as_ref()
            .ok_or_else(|| Status::failed_precondition("call Index first"))?;

        let result = codegraph_graph::queries::impact_of_change(
            graph,
            &req.symbol,
            req.include_tests,
        )
        .ok_or_else(|| Status::not_found(format!("symbol not found: {}", req.symbol)))?;

        let response = ImpactOfChangeResponse {
            symbol: Some(node_to_pb(&result.symbol)),
            affected: result.affected.iter().map(node_to_pb).collect(),
            affected_packages: result.affected_packages.clone(),
            affected_routes: result.affected_routes.iter().map(node_to_pb).collect(),
            total_functions: result.total_functions as u32,
            total_packages: result.total_packages as u32,
        };

        Ok(Response::new(response))
    }

    async fn find_dead_code(
        &self,
        request: Request<FindDeadCodeRequest>,
    ) -> Result<Response<FindDeadCodeResponse>, Status> {
        let req = request.into_inner();
        let state = self.state.read().await;
        let graph = state
            .graph
            .as_ref()
            .ok_or_else(|| Status::failed_precondition("call Index first"))?;

        let result = codegraph_graph::dead_code::find_dead_code(graph, req.include_exported);

        let response = FindDeadCodeResponse {
            dead: result.dead.iter().map(node_to_pb).collect(),
            total: result.total as u32,
        };

        Ok(Response::new(response))
    }

    async fn get_metrics(
        &self,
        request: Request<GetMetricsRequest>,
    ) -> Result<Response<GetMetricsResponse>, Status> {
        let req = request.into_inner();
        let state = self.state.read().await;
        let graph = state
            .graph
            .as_ref()
            .ok_or_else(|| Status::failed_precondition("call Index first"))?;

        let metrics = codegraph_graph::metrics::get_metrics(graph, &req.package);

        let packages = metrics
            .iter()
            .map(|m| PackageMetrics {
                package: m.package.clone(),
                language: language_to_pb(m.language) as i32,
                fan_in: m.fan_in,
                fan_out: m.fan_out,
                instability: m.instability,
                abstractness: m.abstractness,
                distance: m.distance,
                nodes_count: m.nodes_count,
                edges_count: m.edges_count,
            })
            .collect();

        Ok(Response::new(GetMetricsResponse { packages }))
    }

    async fn check_architecture(
        &self,
        _request: Request<CheckArchitectureRequest>,
    ) -> Result<Response<CheckArchitectureResponse>, Status> {
        Err(Status::unimplemented(
            "check_architecture будет реализован в Layer 5",
        ))
    }

    async fn find_type_smells(
        &self,
        _request: Request<FindTypeSmellsRequest>,
    ) -> Result<Response<FindTypeSmellsResponse>, Status> {
        Err(Status::unimplemented(
            "find_type_smells будет реализован в Layer 5",
        ))
    }

    async fn find_complexity_patterns(
        &self,
        request: Request<FindComplexityPatternsRequest>,
    ) -> Result<Response<FindComplexityPatternsResponse>, Status> {
        let req = request.into_inner();
        let state = self.state.read().await;

        let project_path = state
            .project_path
            .as_ref()
            .ok_or_else(|| Status::failed_precondition("call Index first"))?;

        let options = codegraph_parser::complexity::ComplexityOptions {
            min_nesting_level: if req.min_nesting_level == 0 {
                2
            } else {
                req.min_nesting_level
            },
            include_recursion: req.include_recursion,
        };

        let files = codegraph_parser::list_source_files(project_path);
        let mut all_patterns = Vec::new();

        for path in files {
            if path.extension().and_then(|e| e.to_str()) != Some("go") {
                continue;
            }
            let Ok(source) = std::fs::read_to_string(&path) else {
                continue;
            };
            let rel_path = path
                .strip_prefix(project_path)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();

            match codegraph_parser::complexity::find_patterns(&source, &rel_path, &options) {
                Ok(mut patterns) => all_patterns.append(&mut patterns),
                Err(e) => eprintln!("[warn] complexity {}: {}", path.display(), e),
            }
        }

        let diagnostics = all_patterns
            .iter()
            .map(|p| Diagnostic {
                severity: severity_to_pb(p.severity) as i32,
                rule_id: p.rule_id.clone(),
                message: p.message.clone(),
                location: Some(Location {
                    file: p.location.file.clone(),
                    line: p.location.line,
                    column: p.location.column,
                }),
                symbol: p.symbol.clone(),
                suggestion: p.suggestion.clone(),
            })
            .collect::<Vec<_>>();

        Ok(Response::new(FindComplexityPatternsResponse {
            patterns: diagnostics,
            total: all_patterns.len() as u32,
        }))
    }
}

fn node_to_pb(node: &codegraph_ir::Node) -> Node {
    Node {
        id: node.id.clone(),
        kind: node_kind_to_pb(node.kind) as i32,
        name: node.name.clone(),
        qualified_name: node.qualified_name.clone(),
        package: node.package.clone(),
        language: language_to_pb(node.language) as i32,
        location: Some(Location {
            file: node.location.file.clone(),
            line: node.location.line,
            column: node.location.column,
        }),
        exported: node.exported,
        fan_in: node.fan_in,
        fan_out: node.fan_out,
    }
}

fn language_to_pb(lang: Language) -> pb::Language {
    match lang {
        Language::Go => pb::Language::Go,
        Language::Python => pb::Language::Python,
        Language::Rust => pb::Language::Rust,
    }
}

fn node_kind_to_pb(kind: NodeKind) -> pb::NodeKind {
    match kind {
        NodeKind::Function => pb::NodeKind::Function,
        NodeKind::Method => pb::NodeKind::Method,
        NodeKind::Type => pb::NodeKind::Type,
        NodeKind::Interface => pb::NodeKind::Interface,
        NodeKind::Package => pb::NodeKind::Package,
        NodeKind::Route => pb::NodeKind::Route,
        NodeKind::Test => pb::NodeKind::Test,
        NodeKind::Main => pb::NodeKind::Main,
    }
}

#[allow(dead_code)]
fn edge_kind_to_pb(kind: EdgeKind) -> pb::EdgeKind {
    match kind {
        EdgeKind::Calls => pb::EdgeKind::Calls,
        EdgeKind::Imports => pb::EdgeKind::Imports,
        EdgeKind::Implements => pb::EdgeKind::Implements,
        EdgeKind::References => pb::EdgeKind::References,
    }
}

fn severity_to_pb(s: codegraph_ir::Severity) -> pb::diagnostic::Severity {
    match s {
        codegraph_ir::Severity::Info => pb::diagnostic::Severity::Info,
        codegraph_ir::Severity::Warning => pb::diagnostic::Severity::Warning,
        codegraph_ir::Severity::Error => pb::diagnostic::Severity::Error,
    }
}

fn count_source_files(path: &std::path::Path) -> u32 {
    codegraph_parser::list_source_files(path).len() as u32
}