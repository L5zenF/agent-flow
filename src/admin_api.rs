use std::path::PathBuf;
use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Serialize;
use tokio::sync::RwLock;

use crate::config::{
    GatewayConfig, RuntimeState, load_runtime_state, normalize_legacy_rule_graph, parse_config,
    runtime_state_from_config, save_config_atomic,
};
use crate::wasm_plugins::{ManifestCapability, PluginRegistry};

#[derive(Clone)]
pub struct AdminState {
    pub runtime_state: Arc<RwLock<RuntimeState>>,
    pub config_path: PathBuf,
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
    Json(state.runtime_state.read().await.config.clone())
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
    State(state): State<AdminState>,
    Json(candidate): Json<GatewayConfig>,
) -> impl IntoResponse {
    match toml::to_string(&candidate)
        .map_err(|error| error.to_string())
        .and_then(|raw| parse_config(&raw).map_err(|error| error.to_string()))
        .and_then(|normalized| {
            runtime_state_from_config(&state.config_path, normalized)
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
    let runtime_state = match runtime_state_from_config(&state.config_path, normalized.clone()) {
        Ok(runtime_state) => runtime_state,
        Err(error) => return (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    };
    match save_config_atomic(&state.config_path, &normalized).map_err(|error| error.to_string()) {
        Ok(()) => {
            *state.runtime_state.write().await = runtime_state;
            StatusCode::NO_CONTENT.into_response()
        }
        Err(error) => (StatusCode::BAD_REQUEST, error).into_response(),
    }
}

pub async fn reload_config(State(state): State<AdminState>) -> impl IntoResponse {
    match load_runtime_state(&state.config_path).map_err(|error| error.to_string()) {
        Ok(runtime_state) => {
            *state.runtime_state.write().await = runtime_state;
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

#[cfg(test)]
mod tests {
    use super::{AdminState, validate_config_handler};
    use crate::config::{GatewayConfig, parse_config, runtime_state_from_config};
    use crate::wasm_plugins::load_plugin_registry;
    use axum::Json;
    use axum::extract::State;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tokio::sync::RwLock;

    fn temp_dir(name: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be monotonic enough for tests")
            .as_nanos();
        dir.push(format!(
            "proxy-tools-admin-api-{name}-{}-{stamp}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("temp dir should be creatable");
        dir
    }

    fn build_state(root: &std::path::Path) -> AdminState {
        let config_path = root.join("gateway.toml");
        let config = parse_config(
            r#"
listen = "127.0.0.1:9001"
admin_listen = "127.0.0.1:9002"

[rule_graph]
version = 1
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"
position = { x = 0.0, y = 0.0 }
"#,
        )
        .expect("legacy config should parse");
        let runtime_state =
            runtime_state_from_config(&config_path, config).expect("runtime state should build");

        AdminState {
            runtime_state: Arc::new(RwLock::new(runtime_state)),
            config_path,
            plugin_registry: Arc::new(
                load_plugin_registry(&root.join("plugins")).expect("plugin registry should load"),
            ),
        }
    }

    #[tokio::test]
    async fn validate_rejects_missing_indexed_workflow_file() {
        let root = temp_dir("validate-missing-workflow");
        let state = build_state(&root);
        let candidate = GatewayConfig {
            listen: "127.0.0.1:9001".to_string(),
            admin_listen: "127.0.0.1:9002".to_string(),
            default_secret_env: None,
            providers: Vec::new(),
            models: Vec::new(),
            routes: Vec::new(),
            header_rules: Vec::new(),
            rule_graph: None,
            workflows_dir: Some("workflows".to_string()),
            active_workflow_id: Some("chat-routing".to_string()),
            workflows: vec![crate::config::WorkflowIndexEntry {
                id: "chat-routing".to_string(),
                name: "Chat Routing".to_string(),
                file: "chat-routing.toml".to_string(),
                description: None,
            }],
        };

        let response = validate_config_handler(State(state), Json(candidate))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
