use crate::server::AppState;
use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use serde::Deserialize;
use tracing::{error, trace};
use db::models::db_files::Files;

pub async fn get_all_files(
    State(app_state): State<AppState>,
) -> (StatusCode, Json<Vec<Files>>) {
    match app_state.repositories.files_repo.get_all().await {
        Ok(all) => (StatusCode::OK, Json(all)),
        Err(e) => {
            error!("Error fetching all docs: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(vec![]))
        }
    }
}

pub async fn check_file_exists(
    State(app_state): State<AppState>,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> (StatusCode, Json<bool>) {
    if let Some(path) = q.get("path") {
        match app_state.repositories.files_repo.contains_path(path).await {
            Ok(exists) => (StatusCode::OK, Json(exists)),
            Err(e) => {
                error!("Error checking file existence: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, Json(false))
            }
        }
    } else {
        (StatusCode::BAD_REQUEST, Json(false))
    }
}
