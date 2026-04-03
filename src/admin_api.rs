use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use tokio::sync::RwLock;

use crate::config::{normalize_legacy_rule_graph, parse_config, save_config_atomic, GatewayConfig};

#[derive(Clone)]
pub struct AdminState {
    pub config: Arc<RwLock<GatewayConfig>>,
    pub config_path: PathBuf,
}

pub async fn get_config(State(state): State<AdminState>) -> impl IntoResponse {
    Json(state.config.read().await.clone())
}

pub async fn validate_config_handler(
    State(_state): State<AdminState>,
    Json(candidate): Json<GatewayConfig>,
) -> impl IntoResponse {
    match toml::to_string(&candidate)
        .map_err(|error| error.to_string())
        .and_then(|raw| parse_config(&raw).map(|_| ()).map_err(|error| error.to_string()))
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error).into_response(),
    }
}

pub async fn put_config(
    State(state): State<AdminState>,
    Json(candidate): Json<GatewayConfig>,
) -> impl IntoResponse {
    let normalized = normalize_legacy_rule_graph(candidate);
    match save_config_atomic(&state.config_path, &normalized).map_err(|error| error.to_string()) {
        Ok(()) => {
            *state.config.write().await = normalized;
            StatusCode::NO_CONTENT.into_response()
        }
        Err(error) => (StatusCode::BAD_REQUEST, error).into_response(),
    }
}

pub async fn reload_config(State(state): State<AdminState>) -> impl IntoResponse {
    match crate::config::load_config(&state.config_path).map_err(|error| error.to_string()) {
        Ok(config) => {
            *state.config.write().await = config;
            StatusCode::NO_CONTENT.into_response()
        }
        Err(error) => (StatusCode::BAD_REQUEST, error).into_response(),
    }
}
