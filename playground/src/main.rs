mod compile;

use axum::{Json, Router, routing::{get, post}};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{net::SocketAddr, path::PathBuf};
use tower_http::services::ServeDir;

#[derive(Deserialize)]
struct CompileRequest {
    source: String,
}

#[derive(Serialize)]
struct ExampleScript {
    name: String,
    source: String,
}

async fn compile_handler(Json(request): Json<CompileRequest>) -> Json<Value> {
    Json(compile::compile_source(&request.source))
}

async fn examples_handler() -> Json<Vec<ExampleScript>> {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../parser/examples/scripts");
    let mut paths: Vec<_> = std::fs::read_dir(dir)
        .map(|entries| entries.filter_map(Result::ok).map(|entry| entry.path()).collect())
        .unwrap_or_default();
    paths.retain(|path| path.extension().is_some_and(|ext| ext == "qurts"));
    paths.sort();

    let examples = paths
        .into_iter()
        .filter_map(|path| {
            let source = std::fs::read_to_string(&path).ok()?;
            let name = path.file_name()?.to_string_lossy().to_string();
            Some(ExampleScript { name, source })
        })
        .collect();

    Json(examples)
}

#[tokio::main]
async fn main() {
    let frontend_dist = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/frontend/dist"));

    let app = Router::new()
        .route("/api/compile", post(compile_handler))
        .route("/api/examples", get(examples_handler))
        .fallback_service(ServeDir::new(frontend_dist));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = tokio::net::TcpListener::bind(addr).await.expect("failed to bind");
    println!("playground listening on http://{addr}");
    axum::serve(listener, app).await.expect("server error");
}
