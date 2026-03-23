use crate::api::{check_file_exists, get_all_files};
use axum::Router;
use axum::routing::get;
use config::Config;
use std::io::Error;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

#[derive(Clone)]
pub struct AppState {
    pub repositories: Arc<db::repositories::Repositories>,
}

pub async fn init(
    repositories: Arc<db::repositories::Repositories>,
    config: &Config,
) -> Result<(TcpListener, Router), Error> {
    let app_state = AppState {
        repositories: repositories.clone(),
    };

    let api_router = Router::new()
        .route("/files", get(get_all_files))
        .route("/files/exists", get(check_file_exists))
        // .route("/docs", get(get_docs).post(add_doc))
        // .route("/protected", get(protected_route))
        .with_state(app_state.clone());

    let cors_layer = CorsLayer::new()
        .allow_origin(Any) // Open access to selected route
        .allow_methods(Any);

    let app = Router::new()
        .route("/", get(index))
        .route("/health", get(|| async { "OK" }))
        .route("/healthz", get(|| async { "OK" }))
        // .layer(ServiceBuilder::new().layer(cors_layer))
        .nest("/api", api_router)
        .layer(cors_layer)
        .with_state(app_state);

    let listener = TcpListener::bind(format!("0.0.0.0:{}", config.port)).await;

    Ok((listener?, app))
}

async fn index() -> &'static str {
    "Hello, world!"
}
