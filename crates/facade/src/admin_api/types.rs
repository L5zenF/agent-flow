use std::path::PathBuf;
use std::sync::Arc;

use application::SettingsSchema;
use infrastructure::plugin_registry::PluginConfigSchema;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::config::RuntimeState;
use infrastructure::plugin_registry::PluginRegistry;

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

#[derive(Debug, Clone, Deserialize)]
pub struct CreateWorkflowRequest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[allow(dead_code)]
pub type SettingsSchemaResponse = SettingsSchema;
