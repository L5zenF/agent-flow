use application::{
    SettingsSchema, gateway_settings_schema, reload_runtime_state, replace_config,
    validate_candidate_config,
};
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::admin_api::types::AdminState;
use crate::config::{
    GatewayConfig, load_runtime_state, normalize_legacy_rule_graph, parse_config,
    runtime_state_from_config, save_config_atomic,
};

pub async fn get_config(State(state): State<AdminState>) -> impl IntoResponse {
    Json(state.runtime_state.read().await.config.clone())
}

pub async fn get_settings_schema() -> Json<SettingsSchema> {
    Json(gateway_settings_schema())
}

pub async fn validate_config_handler(
    State(state): State<AdminState>,
    Json(candidate): Json<GatewayConfig>,
) -> impl IntoResponse {
    match validate_candidate_config(&candidate, toml::to_string, parse_config, |normalized| {
        runtime_state_from_config(&state.config_path, normalized).map(|_| ())
    }) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

pub async fn put_config(
    State(state): State<AdminState>,
    Json(candidate): Json<GatewayConfig>,
) -> impl IntoResponse {
    match replace_config(
        candidate,
        normalize_legacy_rule_graph,
        |normalized| runtime_state_from_config(&state.config_path, normalized.clone()),
        |normalized| save_config_atomic(&state.config_path, normalized),
    ) {
        Ok(runtime_state) => {
            *state.runtime_state.write().await = runtime_state;
            StatusCode::NO_CONTENT.into_response()
        }
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

pub async fn reload_config(State(state): State<AdminState>) -> impl IntoResponse {
    match reload_runtime_state(|| load_runtime_state(&state.config_path)) {
        Ok(runtime_state) => {
            *state.runtime_state.write().await = runtime_state;
            StatusCode::NO_CONTENT.into_response()
        }
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}
