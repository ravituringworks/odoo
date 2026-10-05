//! HTTP transport (axum). `odoo-server --db odoo.sqlite --modules sale_management,account,stock --port 8069`
use axum::{extract::State, http::StatusCode, routing::{get, post}, Json, Router};
use odoo_app::{App, Config, Request};
use serde_json::{json, Value};
use std::{path::PathBuf, sync::Arc};
use tower_http::cors::CorsLayer;

fn arg(name: &str, default: &str) -> String {
    let a: Vec<String> = std::env::args().collect();
    a.iter().position(|x| x == name).and_then(|i| a.get(i + 1).cloned()).unwrap_or_else(|| default.to_string())
}

async fn rpc(State(app): State<Arc<App>>, Json(req): Json<Request>) -> (StatusCode, Json<Value>) {
    let r = tokio::task::spawn_blocking(move || app.dispatch(req)).await;
    match r {
        Ok(Ok(v)) => (StatusCode::OK, Json(json!({ "result": v }))),
        Ok(Err(e)) => (StatusCode::OK, Json(json!({ "error": { "message": e.to_string(), "kind": format!("{e:?}").split(['(', ' ', '{']).next().unwrap_or("Error") } }))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": { "message": e.to_string() } }))),
    }
}

#[tokio::main]
async fn main() {
    let root = PathBuf::from(arg("--root", env!("CARGO_MANIFEST_DIR"))).join("../..");
    let cfg = Config {
        backend: arg("--backend", "sqlite"), url: arg("--db", "odoo.sqlite"),
        modules: arg("--modules", "sale_management,account,stock,purchase,crm,hr,project,contacts").split(',').map(String::from).collect(),
        schema_dir: root.join("schema"), data_dir: root.join("data"), views_dir: root.join("views"), i18n_dir: root.join("i18n"), seed: arg("--seed", "true") == "true", require_auth: arg("--auth", "true") == "true",
        trial: (arg("--trials", "true") == "true").then(|| odoo_app::trial::TrialConfig { dir: PathBuf::from(arg("--trials-dir", "trials")), max_active: arg("--trials-max", "25").parse().unwrap_or(25), days: arg("--trials-days", "15").parse().unwrap_or(15) }), is_trial: false,
    };
    let app = Arc::new(App::open(&cfg).expect("failed to open application"));
    eprintln!("odoo-rs: {} models from {} modules; backend={}", app.runtime().reg.models.len(), app.runtime().reg.modules.len(), cfg.backend);
    let router = Router::new()
        .route("/api/rpc", post(rpc))
        .route("/api/health", get(|| async { Json(json!({"ok": true})) }))
        .layer(CorsLayer::permissive())
        .with_state(app);
    let addr = format!("127.0.0.1:{}", arg("--port", "8069"));
    let l = tokio::net::TcpListener::bind(&addr).await.expect("bind");
    eprintln!("listening on http://{addr}");
    axum::serve(l, router).await.unwrap();
}
