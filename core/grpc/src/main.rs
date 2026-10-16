//gRPC-сервер CodeGraph.
use std::sync::Arc;

use codegraph_grpc::pb::code_graph_server::CodeGraphServer;
use codegraph_grpc::{CodeGraphService, GraphState, SharedState};
use tokio::sync::RwLock;
use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = std::env::var("CODEGRAPH_GRPC_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:50051".to_string())
        .parse()?;

    let state: SharedState = Arc::new(RwLock::new(GraphState::default()));
    let service = CodeGraphService::new(state);

    eprintln!("[codegraph-core] gRPC server listening on {}", addr);

    Server::builder()
        .add_service(CodeGraphServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}