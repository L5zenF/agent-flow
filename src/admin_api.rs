use std::path::PathBuf;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::config::{
    GatewayConfig, LoadedWorkflowSet, RuleGraphConfig, RuntimeState, WorkflowFileConfig,
    WorkflowIndexEntry, load_runtime_state, normalize_legacy_rule_graph, parse_config,
    resolve_workflow_path, runtime_state_from_config, save_config_atomic,
    save_workflow_file_atomic,
};
use crate::wasm_plugins::{
    ManifestCapability, ManifestCategory, ManifestIcon, ManifestTone, PluginConfigSchema,
    PluginRegistry,
};

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
    pub config_schema: Option<PluginConfigSchema>,
    pub ui: PluginManifestUiSummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct PluginManifestUiSummary {
    pub icon: Option<String>,
    pub category: Option<String>,
    pub tone: Option<String>,
    pub order: Option<i32>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkflowSummary {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub file: String,
    pub is_active: bool,
    pub node_count: usize,
    pub edge_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct SettingsSchema {
    pub global: SettingsSchemaSection,
    pub providers: SettingsSchemaSection,
    pub models: SettingsSchemaSection,
}

#[derive(Debug, Clone, Serialize)]
pub struct SettingsSchemaSection {
    pub key: String,
    pub title: String,
    pub description: String,
    pub list_label: Option<String>,
    pub add_label: Option<String>,
    pub empty_text: Option<String>,
    pub fields: Vec<SettingsSchemaField>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SettingsSchemaField {
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub option_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<SettingsSchemaField>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateWorkflowRequest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
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
            config_schema: plugin.manifest().config_schema.clone(),
            ui: PluginManifestUiSummary {
                icon: plugin
                    .manifest()
                    .ui
                    .icon
                    .as_ref()
                    .map(manifest_icon_name)
                    .map(str::to_string),
                category: plugin
                    .manifest()
                    .ui
                    .category
                    .as_ref()
                    .map(manifest_category_name)
                    .map(str::to_string),
                tone: plugin
                    .manifest()
                    .ui
                    .tone
                    .as_ref()
                    .map(manifest_tone_name)
                    .map(str::to_string),
                order: plugin.manifest().ui.order,
                tags: plugin.manifest().ui.tags.clone(),
            },
        })
        .collect::<Vec<_>>();

    Json(plugins)
}

pub async fn get_settings_schema() -> Json<SettingsSchema> {
    Json(SettingsSchema {
        global: SettingsSchemaSection {
            key: "global".to_string(),
            title: "Global config".to_string(),
            description:
                "These values still use the live gateway config state and stay host-controlled."
                    .to_string(),
            list_label: None,
            add_label: None,
            empty_text: None,
            fields: vec![
                text_field("listen", "Listen"),
                text_field("admin_listen", "Admin Listen"),
                SettingsSchemaField {
                    key: "default_secret_env".to_string(),
                    label: "Default Secret Env".to_string(),
                    field_type: "text".to_string(),
                    required: None,
                    placeholder: Some("PROXY_SECRET".to_string()),
                    help_text: None,
                    option_source: None,
                    fields: None,
                },
            ],
        },
        providers: SettingsSchemaSection {
            key: "providers".to_string(),
            title: "Providers".to_string(),
            description: "Manage upstream providers and their default headers.".to_string(),
            list_label: Some("Providers".to_string()),
            add_label: Some("Add Provider".to_string()),
            empty_text: Some("No providers configured.".to_string()),
            fields: vec![
                text_field("id", "ID"),
                text_field("name", "Name"),
                SettingsSchemaField {
                    key: "base_url".to_string(),
                    label: "Base URL".to_string(),
                    field_type: "text".to_string(),
                    required: Some(true),
                    placeholder: Some("https://example.com".to_string()),
                    help_text: None,
                    option_source: None,
                    fields: None,
                },
                SettingsSchemaField {
                    key: "default_headers".to_string(),
                    label: "Default Headers".to_string(),
                    field_type: "object_list".to_string(),
                    required: None,
                    placeholder: None,
                    help_text: Some("Headers sent with every upstream request.".to_string()),
                    option_source: None,
                    fields: Some(vec![
                        text_field("name", "Header"),
                        text_field("value", "Value"),
                        text_field("secret_env", "Secret Env"),
                        SettingsSchemaField {
                            key: "encrypted".to_string(),
                            label: "Encrypted".to_string(),
                            field_type: "boolean".to_string(),
                            required: None,
                            placeholder: None,
                            help_text: None,
                            option_source: None,
                            fields: None,
                        },
                    ]),
                },
            ],
        },
        models: SettingsSchemaSection {
            key: "models".to_string(),
            title: "Models".to_string(),
            description: "Attach models to providers through the shared config state.".to_string(),
            list_label: Some("Models".to_string()),
            add_label: Some("Add Model".to_string()),
            empty_text: Some("No models configured.".to_string()),
            fields: vec![
                text_field("id", "ID"),
                text_field("name", "Name"),
                SettingsSchemaField {
                    key: "provider_id".to_string(),
                    label: "Provider".to_string(),
                    field_type: "select".to_string(),
                    required: Some(true),
                    placeholder: None,
                    help_text: None,
                    option_source: Some("providers".to_string()),
                    fields: None,
                },
                SettingsSchemaField {
                    key: "description".to_string(),
                    label: "Description".to_string(),
                    field_type: "textarea".to_string(),
                    required: None,
                    placeholder: Some("Optional model description.".to_string()),
                    help_text: None,
                    option_source: None,
                    fields: None,
                },
            ],
        },
    })
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

pub async fn get_workflows(State(state): State<AdminState>) -> Json<Vec<WorkflowSummary>> {
    let runtime_state = state.runtime_state.read().await;
    Json(
        runtime_state
            .config
            .workflows
            .iter()
            .map(|workflow| workflow_summary(workflow, &runtime_state.workflow_set))
            .collect(),
    )
}

pub async fn get_workflow(
    Path(id): Path<String>,
    State(state): State<AdminState>,
) -> Result<Json<WorkflowFileConfig>, (StatusCode, String)> {
    let runtime_state = state.runtime_state.read().await;
    workflow_document(&runtime_state.workflow_set, id.as_str())
        .map(Json)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("workflow '{id}' was not found"),
            )
        })
}

pub async fn create_workflow(
    State(state): State<AdminState>,
    Json(input): Json<CreateWorkflowRequest>,
) -> Result<(StatusCode, Json<WorkflowSummary>), (StatusCode, String)> {
    let id = input.id.trim();
    if id.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "workflow id cannot be empty".to_string(),
        ));
    }
    let name = input.name.trim();
    if name.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "workflow name cannot be empty".to_string(),
        ));
    }

    let mut runtime_state = state.runtime_state.write().await;
    let mut next_config = runtime_state.config.clone();
    if next_config
        .workflows
        .iter()
        .any(|workflow| workflow.id == id)
    {
        return Err((
            StatusCode::CONFLICT,
            format!("workflow '{id}' already exists"),
        ));
    }

    let workflow = WorkflowIndexEntry {
        id: id.to_string(),
        name: name.to_string(),
        file: format!("{id}.toml"),
        description: input.description.and_then(normalize_optional_text),
    };
    if next_config.active_workflow_id.is_none() {
        next_config.active_workflow_id = Some(workflow.id.clone());
    }
    next_config.workflows.push(workflow.clone());
    ensure_active_workflow_file(&state.config_path, &runtime_state).map_err(invalid_request)?;

    let workflow_path = resolve_workflow_path(&state.config_path, &next_config, &workflow.file)
        .map_err(invalid_request)?;
    if workflow_path.exists() {
        return Err((
            StatusCode::CONFLICT,
            format!("workflow file '{}' already exists", workflow_path.display()),
        ));
    }

    let document = default_workflow_document();
    save_workflow_file_atomic(&workflow_path, &document).map_err(invalid_request)?;
    let next_runtime_state =
        match runtime_state_from_config(&state.config_path, next_config.clone()) {
            Ok(next_runtime_state) => next_runtime_state,
            Err(error) => {
                let _ = std::fs::remove_file(&workflow_path);
                return Err(invalid_request(error));
            }
        };
    if let Err(error) = save_config_atomic(&state.config_path, &next_config) {
        let _ = std::fs::remove_file(&workflow_path);
        return Err(invalid_request(error));
    }

    let summary = workflow_summary(&workflow, &next_runtime_state.workflow_set);
    *runtime_state = next_runtime_state;
    Ok((StatusCode::CREATED, Json(summary)))
}

pub async fn put_workflow(
    Path(id): Path<String>,
    State(state): State<AdminState>,
    Json(input): Json<WorkflowFileConfig>,
) -> Result<Json<WorkflowFileConfig>, (StatusCode, String)> {
    let mut runtime_state = state.runtime_state.write().await;
    let workflow = runtime_state
        .config
        .workflows
        .iter()
        .find(|workflow| workflow.id == id)
        .cloned()
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("workflow '{id}' was not found"),
            )
        })?;
    let workflow_path =
        resolve_workflow_path(&state.config_path, &runtime_state.config, &workflow.file)
            .map_err(invalid_request)?;
    let previous_document = workflow_document(&runtime_state.workflow_set, id.as_str());

    save_workflow_file_atomic(&workflow_path, &input).map_err(invalid_request)?;
    let next_runtime_state =
        match runtime_state_from_config(&state.config_path, runtime_state.config.clone()) {
            Ok(next_runtime_state) => next_runtime_state,
            Err(error) => {
                if let Some(previous_document) = previous_document {
                    let _ = save_workflow_file_atomic(&workflow_path, &previous_document);
                }
                return Err(invalid_request(error));
            }
        };

    *runtime_state = next_runtime_state;
    Ok(Json(input))
}

pub async fn activate_workflow(
    Path(id): Path<String>,
    State(state): State<AdminState>,
) -> Result<Json<WorkflowSummary>, (StatusCode, String)> {
    let mut runtime_state = state.runtime_state.write().await;
    let workflow = runtime_state
        .config
        .workflows
        .iter()
        .find(|workflow| workflow.id == id)
        .cloned()
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("workflow '{id}' was not found"),
            )
        })?;

    let mut next_config = runtime_state.config.clone();
    next_config.active_workflow_id = Some(id.clone());
    ensure_active_workflow_file(&state.config_path, &runtime_state).map_err(invalid_request)?;
    let next_runtime_state = runtime_state_from_config(&state.config_path, next_config.clone())
        .map_err(invalid_request)?;
    save_config_atomic(&state.config_path, &next_config).map_err(invalid_request)?;

    let summary = workflow_summary(&workflow, &next_runtime_state.workflow_set);
    *runtime_state = next_runtime_state;
    Ok(Json(summary))
}

fn manifest_capability_name(capability: &ManifestCapability) -> &'static str {
    match capability {
        ManifestCapability::Log => "log",
        ManifestCapability::Fs => "fs",
        ManifestCapability::Network => "network",
    }
}

fn manifest_icon_name(icon: &ManifestIcon) -> &'static str {
    match icon {
        ManifestIcon::Puzzle => "puzzle",
        ManifestIcon::Split => "split",
        ManifestIcon::Route => "route",
        ManifestIcon::Wand => "wand",
        ManifestIcon::Shield => "shield",
        ManifestIcon::Code => "code",
        ManifestIcon::Filter => "filter",
        ManifestIcon::Database => "database",
        ManifestIcon::FileText => "file_text",
    }
}

fn manifest_category_name(category: &ManifestCategory) -> &'static str {
    match category {
        ManifestCategory::Control => "control",
        ManifestCategory::Transform => "transform",
        ManifestCategory::Routing => "routing",
        ManifestCategory::Policy => "policy",
        ManifestCategory::Utility => "utility",
    }
}

fn manifest_tone_name(tone: &ManifestTone) -> &'static str {
    match tone {
        ManifestTone::Slate => "slate",
        ManifestTone::Blue => "blue",
        ManifestTone::Sky => "sky",
        ManifestTone::Teal => "teal",
        ManifestTone::Emerald => "emerald",
        ManifestTone::Amber => "amber",
        ManifestTone::Rose => "rose",
        ManifestTone::Violet => "violet",
    }
}

fn text_field(key: &str, label: &str) -> SettingsSchemaField {
    SettingsSchemaField {
        key: key.to_string(),
        label: label.to_string(),
        field_type: "text".to_string(),
        required: Some(true),
        placeholder: None,
        help_text: None,
        option_source: None,
        fields: None,
    }
}

fn workflow_document(workflows: &LoadedWorkflowSet, id: &str) -> Option<WorkflowFileConfig> {
    workflows.by_id.get(id).cloned().or_else(|| {
        (workflows.active_workflow_id.as_deref() == Some(id))
            .then(|| workflows.active_graph().cloned())
            .flatten()
            .map(|workflow| WorkflowFileConfig { workflow })
    })
}

fn workflow_summary(
    workflow: &WorkflowIndexEntry,
    workflows: &LoadedWorkflowSet,
) -> WorkflowSummary {
    let graph =
        workflow_document(workflows, workflow.id.as_str()).map(|document| document.workflow);
    WorkflowSummary {
        id: workflow.id.clone(),
        name: workflow.name.clone(),
        description: workflow.description.clone(),
        file: workflow.file.clone(),
        is_active: workflows.active_workflow_id.as_deref() == Some(workflow.id.as_str()),
        node_count: graph.as_ref().map_or(0, |graph| graph.nodes.len()),
        edge_count: graph.as_ref().map_or(0, |graph| graph.edges.len()),
    }
}

fn ensure_active_workflow_file(
    config_path: &std::path::Path,
    runtime_state: &RuntimeState,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(active_workflow_id) = runtime_state.config.active_workflow_id.as_deref() else {
        return Ok(());
    };
    if runtime_state
        .workflow_set
        .by_id
        .contains_key(active_workflow_id)
    {
        return Ok(());
    }

    let Some(active_graph) = runtime_state.workflow_set.active_graph() else {
        return Ok(());
    };
    let Some(workflow) = runtime_state
        .config
        .workflows
        .iter()
        .find(|workflow| workflow.id == active_workflow_id)
    else {
        return Ok(());
    };

    let workflow_path = resolve_workflow_path(config_path, &runtime_state.config, &workflow.file)?;
    save_workflow_file_atomic(
        &workflow_path,
        &WorkflowFileConfig {
            workflow: active_graph.clone(),
        },
    )
}

fn default_workflow_document() -> WorkflowFileConfig {
    WorkflowFileConfig {
        workflow: RuleGraphConfig {
            version: 1,
            start_node_id: "start".to_string(),
            nodes: vec![crate::config::RuleGraphNode {
                id: "start".to_string(),
                node_type: crate::config::RuleGraphNodeType::Start,
                position: crate::config::GraphPosition { x: 0.0, y: 0.0 },
                note: None,
                condition: None,
                route_provider: None,
                select_model: None,
                rewrite_path: None,
                set_context: None,
                router: None,
                log: None,
                set_header: None,
                remove_header: None,
                copy_header: None,
                set_header_if_absent: None,
                note_node: None,
                wasm_plugin: None,
                match_node: None,
                code_runner: None,
            }],
            edges: Vec::new(),
        },
    }
}

fn normalize_optional_text(value: String) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn invalid_request(error: impl ToString) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        AdminState, activate_workflow, create_workflow, get_workflows, put_workflow,
        validate_config_handler,
    };
    use crate::config::{GatewayConfig, parse_config, runtime_state_from_config};
    use crate::wasm_plugins::load_plugin_registry;
    use axum::Json;
    use axum::extract::Path;
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

    fn write_workflow(root: &std::path::Path, file: &str, node_count: usize, edge_count: usize) {
        let workflows_dir = root.join("workflows");
        fs::create_dir_all(&workflows_dir).expect("workflows dir should be creatable");

        let mut raw = String::from(
            r#"[workflow]
version = 1
start_node_id = "start"

[[workflow.nodes]]
id = "start"
type = "start"
position = { x = 0.0, y = 0.0 }
"#,
        );

        for index in 1..node_count {
            raw.push_str(&format!(
                r#"

[[workflow.nodes]]
id = "node-{index}"
type = "end"
position = {{ x = {x}, y = 0.0 }}
"#,
                x = (index as f64) * 120.0
            ));
        }

        for index in 0..edge_count {
            let target = if index + 1 < node_count {
                format!("node-{}", index + 1)
            } else {
                "start".to_string()
            };
            let source = if index == 0 {
                "start".to_string()
            } else {
                format!("node-{index}")
            };
            raw.push_str(&format!(
                r#"

[[workflow.edges]]
id = "edge-{index}"
source = "{source}"
target = "{target}"
"#
            ));
        }

        fs::write(workflows_dir.join(file), raw).expect("workflow file should be writable");
    }

    fn build_indexed_state(root: &std::path::Path) -> AdminState {
        let config_path = root.join("gateway.toml");
        write_workflow(root, "chat-routing.toml", 2, 1);
        write_workflow(root, "fallback.toml", 3, 2);
        let config = parse_config(
            r#"
listen = "127.0.0.1:9001"
admin_listen = "127.0.0.1:9002"
workflows_dir = "workflows"
active_workflow_id = "chat-routing"

[[workflows]]
id = "chat-routing"
name = "Chat Routing"
file = "chat-routing.toml"
description = "Primary chat flow"

[[workflows]]
id = "fallback"
name = "Fallback"
file = "fallback.toml"
description = "Fallback workflow"
"#,
        )
        .expect("indexed config should parse");
        fs::write(
            &config_path,
            toml::to_string_pretty(&config).expect("config should serialize"),
        )
        .expect("config file should be writable");

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

    #[tokio::test]
    async fn lists_workflow_gallery_summaries() {
        let root = temp_dir("workflow-gallery");
        let state = build_indexed_state(&root);

        let Json(summaries) = get_workflows(State(state)).await;

        assert_eq!(summaries.len(), 2);
        assert_eq!(summaries[0].id, "chat-routing");
        assert!(summaries[0].is_active);
        assert_eq!(summaries[0].node_count, 2);
        assert_eq!(summaries[0].edge_count, 1);
        assert_eq!(summaries[1].id, "fallback");
        assert!(!summaries[1].is_active);
        assert_eq!(summaries[1].node_count, 3);
        assert_eq!(summaries[1].edge_count, 2);
    }

    #[tokio::test]
    async fn activates_selected_workflow() {
        let root = temp_dir("activate-workflow");
        let state = build_indexed_state(&root);

        let Json(summary) = activate_workflow(Path("fallback".to_string()), State(state.clone()))
            .await
            .expect("activation should succeed");

        assert_eq!(summary.id, "fallback");
        assert!(summary.is_active);
        let runtime_state = state.runtime_state.read().await;
        assert_eq!(
            runtime_state.config.active_workflow_id.as_deref(),
            Some("fallback")
        );
        assert_eq!(
            runtime_state.workflow_set.active_workflow_id.as_deref(),
            Some("fallback")
        );

        let persisted = fs::read_to_string(root.join("gateway.toml"))
            .expect("config file should remain readable");
        assert!(persisted.contains("active_workflow_id = \"fallback\""));
    }

    #[tokio::test]
    async fn create_workflow_persists_file_and_updates_runtime_state() {
        let root = temp_dir("create-workflow");
        let state = build_indexed_state(&root);

        let summary = create_workflow(
            State(state.clone()),
            Json(crate::admin_api::CreateWorkflowRequest {
                id: "new-flow".to_string(),
                name: "New Flow".to_string(),
                description: Some("Created in admin API".to_string()),
            }),
        )
        .await
        .expect("create should succeed")
        .1
        .0;

        assert_eq!(summary.id, "new-flow");
        assert!(!summary.is_active);
        let persisted_workflow = fs::read_to_string(root.join("workflows/new-flow.toml"))
            .expect("new workflow file should be written");
        assert!(persisted_workflow.contains("[workflow]"));
        assert!(persisted_workflow.contains("start_node_id = \"start\""));

        let runtime_state = state.runtime_state.read().await;
        assert!(
            runtime_state
                .config
                .workflows
                .iter()
                .any(|workflow| workflow.id == "new-flow")
        );
        assert!(runtime_state.workflow_set.by_id.contains_key("new-flow"));
    }

    #[tokio::test]
    async fn create_workflow_materializes_legacy_active_graph() {
        let root = temp_dir("create-workflow-legacy");
        let state = build_state(&root);

        let _ = create_workflow(
            State(state.clone()),
            Json(crate::admin_api::CreateWorkflowRequest {
                id: "new-flow".to_string(),
                name: "New Flow".to_string(),
                description: None,
            }),
        )
        .await
        .expect("create should succeed for legacy configs");

        let default_workflow = fs::read_to_string(root.join("workflows/default.toml"))
            .expect("legacy active workflow should be materialized");
        assert!(default_workflow.contains("start_node_id = \"start\""));

        let runtime_state = state.runtime_state.read().await;
        assert_eq!(runtime_state.config.workflows.len(), 2);
        assert!(runtime_state.workflow_set.by_id.contains_key("default"));
        assert!(runtime_state.workflow_set.by_id.contains_key("new-flow"));
    }

    #[tokio::test]
    async fn put_workflow_persists_file_contents() {
        let root = temp_dir("put-workflow");
        let state = build_indexed_state(&root);

        let saved = put_workflow(
            Path("fallback".to_string()),
            State(state.clone()),
            Json(crate::config::WorkflowFileConfig {
                workflow: crate::config::RuleGraphConfig {
                    version: 1,
                    start_node_id: "start".to_string(),
                    nodes: vec![
                        crate::config::RuleGraphNode {
                            id: "start".to_string(),
                            node_type: crate::config::RuleGraphNodeType::Start,
                            position: crate::config::GraphPosition { x: 0.0, y: 0.0 },
                            note: None,
                            condition: None,
                            route_provider: None,
                            select_model: None,
                            rewrite_path: None,
                            set_context: None,
                            router: None,
                            log: None,
                            set_header: None,
                            remove_header: None,
                            copy_header: None,
                            set_header_if_absent: None,
                            note_node: None,
                            wasm_plugin: None,
                            match_node: None,
                            code_runner: None,
                        },
                        crate::config::RuleGraphNode {
                            id: "end".to_string(),
                            node_type: crate::config::RuleGraphNodeType::End,
                            position: crate::config::GraphPosition { x: 120.0, y: 0.0 },
                            note: None,
                            condition: None,
                            route_provider: None,
                            select_model: None,
                            rewrite_path: None,
                            set_context: None,
                            router: None,
                            log: None,
                            set_header: None,
                            remove_header: None,
                            copy_header: None,
                            set_header_if_absent: None,
                            note_node: None,
                            wasm_plugin: None,
                            match_node: None,
                            code_runner: None,
                        },
                    ],
                    edges: vec![crate::config::RuleGraphEdge {
                        id: "edge-1".to_string(),
                        source: "start".to_string(),
                        target: "end".to_string(),
                        source_handle: None,
                    }],
                },
            }),
        )
        .await
        .expect("save should succeed")
        .0;

        assert_eq!(saved.workflow.nodes.len(), 2);
        let persisted = fs::read_to_string(root.join("workflows/fallback.toml"))
            .expect("workflow file should remain readable");
        assert!(persisted.contains("id = \"end\""));

        let runtime_state = state.runtime_state.read().await;
        let workflow = runtime_state
            .workflow_set
            .by_id
            .get("fallback")
            .expect("updated workflow should be loaded");
        assert_eq!(workflow.workflow.nodes.len(), 2);
        assert_eq!(workflow.workflow.edges.len(), 1);
    }
}
