use std::collections::BTreeSet;
use std::error::Error;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::paths::invalid_data;

type PluginResult<T> = Result<T, Box<dyn Error>>;

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

#[derive(Debug, Deserialize)]
struct RawPluginManifest {
    id: String,
    name: String,
    version: String,
    description: String,
    #[serde(default)]
    runtime: Option<String>,
    supported_output_ports: Vec<String>,
    #[serde(default)]
    capabilities: Vec<String>,
    #[serde(default)]
    default_config_schema_hints: Option<toml::Value>,
    #[serde(default)]
    config_schema: Option<PluginConfigSchema>,
    #[serde(default)]
    ui: RawPluginManifestUi,
}

#[derive(Debug, Deserialize, Default)]
struct RawPluginManifestUi {
    #[serde(default)]
    icon: Option<String>,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    tone: Option<String>,
    #[serde(default)]
    order: Option<i32>,
    #[serde(default)]
    tags: Vec<String>,
}

pub(crate) fn parse_plugin_manifest(path: &Path, raw: &str) -> PluginResult<PluginManifest> {
    let raw_manifest: RawPluginManifest = toml::from_str(raw).map_err(|error| {
        invalid_data(format!(
            "failed to parse plugin manifest '{}': {error}",
            path.display()
        ))
    })?;

    let capabilities = raw_manifest
        .capabilities
        .into_iter()
        .map(|capability| parse_manifest_capability(path, &capability))
        .collect::<PluginResult<Vec<_>>>()?;
    let supported_output_ports =
        validate_supported_output_ports(path, raw_manifest.supported_output_ports)?;
    let ui = parse_manifest_ui(path, raw_manifest.ui)?;

    Ok(PluginManifest {
        id: raw_manifest.id,
        name: raw_manifest.name,
        version: raw_manifest.version,
        description: raw_manifest.description,
        runtime: parse_plugin_runtime(path, raw_manifest.runtime.as_deref())?,
        supported_output_ports,
        capabilities,
        default_config_schema_hints: raw_manifest.default_config_schema_hints,
        config_schema: raw_manifest.config_schema,
        ui,
    })
}

fn parse_manifest_capability(path: &Path, capability: &str) -> PluginResult<ManifestCapability> {
    match capability {
        "log" => Ok(ManifestCapability::Log),
        "fs" => Ok(ManifestCapability::Fs),
        "network" => Ok(ManifestCapability::Network),
        other => Err(invalid_data(format!(
            "plugin manifest '{}' declares unsupported capability '{}' (supported: log, fs, network)",
            path.display(),
            other
        ))),
    }
}

fn parse_plugin_runtime(path: &Path, runtime: Option<&str>) -> PluginResult<PluginRuntimeKind> {
    match runtime.map(str::trim).filter(|value| !value.is_empty()) {
        None => Ok(PluginRuntimeKind::Component),
        Some("component") => Ok(PluginRuntimeKind::Component),
        Some("core") => Ok(PluginRuntimeKind::Core),
        Some(other) => Err(invalid_data(format!(
            "plugin manifest '{}' declares unsupported runtime '{}' (supported: component, core)",
            path.display(),
            other
        ))),
    }
}

fn parse_manifest_ui(path: &Path, raw_ui: RawPluginManifestUi) -> PluginResult<PluginManifestUi> {
    let icon = raw_ui
        .icon
        .as_deref()
        .map(|value| parse_manifest_icon(path, value))
        .transpose()?;
    let category = raw_ui
        .category
        .as_deref()
        .map(|value| parse_manifest_category(path, value))
        .transpose()?;
    let tone = raw_ui
        .tone
        .as_deref()
        .map(|value| parse_manifest_tone(path, value))
        .transpose()?;
    let tags = raw_ui
        .tags
        .into_iter()
        .map(|tag| normalize_manifest_tag(path, tag))
        .collect::<PluginResult<Vec<_>>>()?;

    Ok(PluginManifestUi {
        icon,
        category,
        tone,
        order: raw_ui.order,
        tags,
    })
}

fn parse_manifest_icon(path: &Path, value: &str) -> PluginResult<ManifestIcon> {
    match value {
        "puzzle" => Ok(ManifestIcon::Puzzle),
        "split" => Ok(ManifestIcon::Split),
        "route" => Ok(ManifestIcon::Route),
        "wand" => Ok(ManifestIcon::Wand),
        "shield" => Ok(ManifestIcon::Shield),
        "code" => Ok(ManifestIcon::Code),
        "filter" => Ok(ManifestIcon::Filter),
        "database" => Ok(ManifestIcon::Database),
        "file_text" => Ok(ManifestIcon::FileText),
        other => Err(invalid_data(format!(
            "plugin manifest '{}' declares unsupported ui.icon '{}' (supported: puzzle, split, route, wand, shield, code, filter, database, file_text)",
            path.display(),
            other
        ))),
    }
}

fn parse_manifest_category(path: &Path, value: &str) -> PluginResult<ManifestCategory> {
    match value {
        "control" => Ok(ManifestCategory::Control),
        "transform" => Ok(ManifestCategory::Transform),
        "routing" => Ok(ManifestCategory::Routing),
        "policy" => Ok(ManifestCategory::Policy),
        "utility" => Ok(ManifestCategory::Utility),
        other => Err(invalid_data(format!(
            "plugin manifest '{}' declares unsupported ui.category '{}' (supported: control, transform, routing, policy, utility)",
            path.display(),
            other
        ))),
    }
}

fn parse_manifest_tone(path: &Path, value: &str) -> PluginResult<ManifestTone> {
    match value {
        "slate" => Ok(ManifestTone::Slate),
        "blue" => Ok(ManifestTone::Blue),
        "sky" => Ok(ManifestTone::Sky),
        "teal" => Ok(ManifestTone::Teal),
        "emerald" => Ok(ManifestTone::Emerald),
        "amber" => Ok(ManifestTone::Amber),
        "rose" => Ok(ManifestTone::Rose),
        "violet" => Ok(ManifestTone::Violet),
        other => Err(invalid_data(format!(
            "plugin manifest '{}' declares unsupported ui.tone '{}' (supported: slate, blue, sky, teal, emerald, amber, rose, violet)",
            path.display(),
            other
        ))),
    }
}

fn normalize_manifest_tag(path: &Path, tag: String) -> PluginResult<String> {
    let normalized = tag.trim();
    if normalized.is_empty() {
        return Err(invalid_data(format!(
            "plugin manifest '{}' contains an empty ui.tags entry",
            path.display()
        )));
    }
    Ok(normalized.to_owned())
}

fn validate_supported_output_ports(path: &Path, ports: Vec<String>) -> PluginResult<Vec<String>> {
    let mut seen = BTreeSet::new();
    let mut validated = Vec::with_capacity(ports.len());

    for port in ports {
        let normalized = port.trim();
        if normalized.is_empty() {
            return Err(invalid_data(format!(
                "plugin manifest '{}' contains an empty supported_output_ports entry",
                path.display()
            )));
        }
        if !seen.insert(normalized.to_owned()) {
            return Err(invalid_data(format!(
                "plugin manifest '{}' declares duplicate supported_output_port '{}'",
                path.display(),
                normalized
            )));
        }
        validated.push(normalized.to_owned());
    }

    Ok(validated)
}

fn default_config_schema_version() -> u32 {
    1
}
