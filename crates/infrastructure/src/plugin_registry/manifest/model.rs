use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManifestCapability {
    Log,
    Fs,
    Network,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManifestIcon {
    Puzzle,
    Split,
    Route,
    Wand,
    Shield,
    Code,
    Filter,
    Database,
    FileText,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManifestCategory {
    Control,
    Transform,
    Routing,
    Policy,
    Utility,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManifestTone {
    Slate,
    Blue,
    Sky,
    Teal,
    Emerald,
    Amber,
    Rose,
    Violet,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PluginManifestUi {
    pub icon: Option<ManifestIcon>,
    pub category: Option<ManifestCategory>,
    pub tone: Option<ManifestTone>,
    pub order: Option<i32>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginConfigFieldType {
    Text,
    Textarea,
    Select,
    Boolean,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginConfigFieldDataSource {
    Providers,
    Models,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginConfigField {
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: PluginConfigFieldType,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub placeholder: Option<String>,
    #[serde(default)]
    pub help_text: Option<String>,
    #[serde(default)]
    pub data_source: Option<PluginConfigFieldDataSource>,
    #[serde(default)]
    pub depends_on: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginConfigSchema {
    #[serde(default = "default_config_schema_version")]
    pub version: u32,
    #[serde(default)]
    pub fields: Vec<PluginConfigField>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PluginRuntimeKind {
    #[default]
    Component,
    Core,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(default)]
    pub runtime: PluginRuntimeKind,
    pub supported_output_ports: Vec<String>,
    #[serde(default)]
    pub capabilities: Vec<ManifestCapability>,
    #[serde(default)]
    pub default_config_schema_hints: Option<toml::Value>,
    #[serde(default)]
    pub config_schema: Option<PluginConfigSchema>,
    #[serde(default)]
    pub ui: PluginManifestUi,
}

fn default_config_schema_version() -> u32 {
    1
}
