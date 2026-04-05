use std::path::PathBuf;
use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Serialize;
use tokio::sync::RwLock;

use crate::config::{
    GatewayConfig, LoadedWorkflowSet, load_workflow_set, normalize_legacy_rule_graph, parse_config,
    save_config_atomic,
};
use crate::wasm_plugins::{ManifestCapability, PluginRegistry};

#[derive(Clone)]
pub struct AdminState {
    pub config: Arc<RwLock<GatewayConfig>>,
    pub config_path: PathBuf,
    pub workflow_store: Arc<RwLock<LoadedWorkflowSet>>,
    pub plugin_registry: Arc<PluginRegistry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PluginManifestSummary {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub supported_output_ports: Vec<String>,
    pub capabilities: Vec<String>,
    pub default_config_schema_hints: Option<toml::Value>,
}

pub async fn get_config(State(state): State<AdminState>) -> impl IntoResponse {
    Json(state.config.read().await.clone())
}

pub async fn get_plugins(State(state): State<AdminState>) -> impl IntoResponse {
    let plugins = state
        .plugin_registry
        .plugins()
        .map(|plugin| PluginManifestSummary {
            id: plugin.manifest().id.clone(),
            name: plugin.manifest().name.clone(),
            version: plugin.manifest().version.clone(),
            description: plugin.manifest().description.clone(),
            supported_output_ports: plugin.manifest().supported_output_ports.clone(),
            capabilities: plugin
                .manifest()
                .capabilities
                .iter()
                .map(manifest_capability_name)
                .map(str::to_string)
                .collect(),
            default_config_schema_hints: plugin.manifest().default_config_schema_hints.clone(),
        })
        .collect::<Vec<_>>();

    Json(plugins)
}

pub async fn validate_config_handler(
    State(_state): State<AdminState>,
    Json(candidate): Json<GatewayConfig>,
) -> impl IntoResponse {
    match toml::to_string(&candidate)
        .map_err(|error| error.to_string())
        .and_then(|raw| {
            parse_config(&raw)
                .map(|_| ())
                .map_err(|error| error.to_string())
        }) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error).into_response(),
    }
}

pub async fn put_config(
    State(state): State<AdminState>,
    Json(candidate): Json<GatewayConfig>,
) -> impl IntoResponse {
    let normalized = normalize_legacy_rule_graph(candidate);
    let workflow_store = match load_workflow_set(&state.config_path, &normalized) {
        Ok(workflow_store) => workflow_store,
        Err(error) => return (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    };
    match save_config_atomic(&state.config_path, &normalized).map_err(|error| error.to_string()) {
        Ok(()) => {
            *state.config.write().await = normalized;
            *state.workflow_store.write().await = workflow_store;
            StatusCode::NO_CONTENT.into_response()
        }
        Err(error) => (StatusCode::BAD_REQUEST, error).into_response(),
    }
}

pub async fn reload_config(State(state): State<AdminState>) -> impl IntoResponse {
    match crate::config::load_config(&state.config_path).map_err(|error| error.to_string()) {
        Ok(config) => {
            let workflow_store = match load_workflow_set(&state.config_path, &config) {
                Ok(workflow_store) => workflow_store,
                Err(error) => {
                    return (StatusCode::BAD_REQUEST, error.to_string()).into_response();
                }
            };
            *state.config.write().await = config;
            *state.workflow_store.write().await = workflow_store;
            StatusCode::NO_CONTENT.into_response()
        }
        Err(error) => (StatusCode::BAD_REQUEST, error).into_response(),
    }
}

fn manifest_capability_name(capability: &ManifestCapability) -> &'static str {
    match capability {
        ManifestCapability::Log => "log",
        ManifestCapability::Fs => "fs",
        ManifestCapability::Network => "network",
    }
}
