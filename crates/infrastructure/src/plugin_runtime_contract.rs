use std::collections::{HashMap, HashSet};

use crate::gateway_config::{
    ModelConfig, ProviderConfig, WasmCapability, WasmPluginNodeConfig,
};
use serde::{Deserialize, Serialize};

use crate::plugin_registry::{LoadedPlugin, ManifestCapability};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeRequestHeader {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RuntimeCapabilityKind {
    Log,
    Fs,
    Network,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeCapabilityDeclaration {
    pub kind: RuntimeCapabilityKind,
    pub required: bool,
    pub scope: Option<String>,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeCapabilityGrant {
    pub kind: RuntimeCapabilityKind,
    pub allowed: bool,
    pub scope: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub supported_output_ports: Vec<String>,
    pub default_config_schema_hints_json: Option<String>,
    pub capabilities: Vec<RuntimeCapabilityDeclaration>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeNodeConfig {
    pub manifest: RuntimePluginManifest,
    pub grants: Vec<RuntimeCapabilityGrant>,
    pub config_json: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeExecuteInput {
    pub request_method: String,
    pub current_path: String,
    pub request_headers: Vec<RuntimeRequestHeader>,
    pub workflow_context: Vec<(String, String)>,
    pub selected_provider_id: String,
    pub selected_model_id: String,
    pub node_config: RuntimeNodeConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeContextPatchOp {
    Set { key: String, value: String },
    Remove { key: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeHeaderOp {
    Set { name: String, value: String },
    Append { name: String, value: String },
    Remove { name: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeLogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeLogEntry {
    pub level: RuntimeLogLevel,
    pub message: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuntimeExecuteOutput {
    pub context_ops: Vec<RuntimeContextPatchOp>,
    pub header_ops: Vec<RuntimeHeaderOp>,
    pub path_rewrite: Option<String>,
    pub next_port: Option<String>,
    pub logs: Vec<RuntimeLogEntry>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeRunnerInput<'a> {
    pub method: &'a str,
    pub path: &'a str,
    pub headers: std::collections::BTreeMap<String, String>,
    pub context: std::collections::BTreeMap<String, String>,
    pub provider: Option<CodeRunnerProvider<'a>>,
    pub model: Option<CodeRunnerModel<'a>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeRunnerProvider<'a> {
    pub id: &'a str,
    pub name: &'a str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeRunnerModel<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub provider_id: &'a str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreCodeRunnerRequest<'a> {
    pub code: String,
    pub input: CodeRunnerInput<'a>,
}

#[derive(Debug, Deserialize)]
pub struct CoreCodeRunnerResponse {
    pub ok: bool,
    #[serde(default)]
    pub json: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeRunnerOutput {
    #[serde(default)]
    pub context_patch: Vec<CodeRunnerContextPatchOp>,
    #[serde(default)]
    pub header_ops: Vec<CodeRunnerHeaderOp>,
    #[serde(default)]
    pub path_rewrite: Option<String>,
    #[serde(default)]
    pub next_port: Option<String>,
    #[serde(default)]
    pub logs: Vec<CodeRunnerLogEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum CodeRunnerContextPatchOp {
    Set { key: String, value: String },
    Remove { key: String },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum CodeRunnerHeaderOp {
    Set { name: String, value: String },
    Append { name: String, value: String },
    Remove { name: String },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CodeRunnerLogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Deserialize)]
pub struct CodeRunnerLogEntry {
    pub level: CodeRunnerLogLevel,
    pub message: String,
}

pub fn current_request_headers(
    incoming_headers: &axum::http::HeaderMap,
    outgoing_headers: &HashMap<String, Vec<String>>,
) -> Vec<RuntimeRequestHeader> {
    let mut merged = incoming_headers
        .iter()
        .filter_map(|(name, value)| {
            value.to_str().ok().map(|value| RuntimeRequestHeader {
                name: name.as_str().to_string(),
                value: value.to_string(),
            })
        })
        .collect::<Vec<_>>();

    for (name, values) in outgoing_headers {
        merged.retain(|header| !header.name.eq_ignore_ascii_case(name));
        for value in values {
            merged.push(RuntimeRequestHeader {
                name: name.clone(),
                value: value.clone(),
            });
        }
    }

    merged
}

pub fn build_runtime_execute_input(
    plugin: &LoadedPlugin,
    node_id: &str,
    node_config: &WasmPluginNodeConfig,
    method: &str,
    current_path: &str,
    request_headers: &[RuntimeRequestHeader],
    workflow_context: &HashMap<String, String>,
    selected_provider: Option<&ProviderConfig>,
    selected_model: Option<&ModelConfig>,
) -> Result<RuntimeExecuteInput, String> {
    let granted_capabilities = node_config
        .granted_capabilities
        .iter()
        .copied()
        .collect::<HashSet<_>>();
    let manifest = plugin.manifest();
    let mut capability_declarations = Vec::new();
    let mut capability_grants = Vec::new();
    let mut seen_grants = HashSet::new();

    for capability in &manifest.capabilities {
        let kind = runtime_capability_from_manifest(capability);
        let grant_kind = wasm_capability_from_manifest(capability);
        let allowed = granted_capabilities.contains(&grant_kind);
        capability_declarations.push(RuntimeCapabilityDeclaration {
            kind: kind.clone(),
            required: true,
            scope: None,
            description: format!(
                "{} capability required by plugin manifest",
                runtime_capability_name(&kind)
            ),
        });
        capability_grants.push(RuntimeCapabilityGrant {
            kind: kind.clone(),
            allowed,
            scope: capability_scope(grant_kind, node_config),
        });
        seen_grants.insert(kind.clone());

        if !allowed {
            return Err(format!(
                "plugin '{}' requires capability '{}' but node '{}' denied it",
                plugin.plugin_id(),
                runtime_capability_name(&kind),
                node_id
            ));
        }
    }

    for grant in &node_config.granted_capabilities {
        if !manifest
            .capabilities
            .iter()
            .any(|capability| wasm_capability_from_manifest(capability) == *grant)
        {
            return Err(format!(
                "plugin '{}' does not declare capability '{}' but node '{}' granted it",
                plugin.plugin_id(),
                runtime_capability_name(&runtime_capability_from_wasm(*grant)),
                node_id
            ));
        }
        let kind = runtime_capability_from_wasm(*grant);
        if seen_grants.insert(kind.clone()) {
            capability_grants.push(RuntimeCapabilityGrant {
                kind,
                allowed: true,
                scope: capability_scope(*grant, node_config),
            });
        }
    }

    Ok(RuntimeExecuteInput {
        request_method: method.to_string(),
        current_path: current_path.to_string(),
        request_headers: request_headers.to_vec(),
        workflow_context: workflow_context
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
        selected_provider_id: selected_provider
            .map(|provider| provider.id.clone())
            .unwrap_or_default(),
        selected_model_id: selected_model
            .map(|model| model.id.clone())
            .unwrap_or_default(),
        node_config: RuntimeNodeConfig {
            manifest: RuntimePluginManifest {
                id: manifest.id.clone(),
                name: manifest.name.clone(),
                version: manifest.version.clone(),
                description: manifest.description.clone(),
                supported_output_ports: manifest.supported_output_ports.clone(),
                default_config_schema_hints_json: manifest
                    .default_config_schema_hints
                    .as_ref()
                    .map(toml_value_to_json),
                capabilities: capability_declarations,
            },
            grants: capability_grants,
            config_json: (!node_config.config.is_empty())
                .then(|| toml_value_to_json(&toml::Value::Table(node_config.config.clone()))),
        },
    })
}

pub fn build_code_runner_input(input: &RuntimeExecuteInput) -> CodeRunnerInput<'_> {
    CodeRunnerInput {
        method: input.request_method.as_str(),
        path: input.current_path.as_str(),
        headers: input
            .request_headers
            .iter()
            .map(|header| (header.name.to_ascii_lowercase(), header.value.clone()))
            .collect(),
        context: input.workflow_context.iter().cloned().collect(),
        provider: (!input.selected_provider_id.is_empty()).then(|| CodeRunnerProvider {
            id: input.selected_provider_id.as_str(),
            name: input.selected_provider_id.as_str(),
        }),
        model: (!input.selected_model_id.is_empty()).then(|| CodeRunnerModel {
            id: input.selected_model_id.as_str(),
            name: input.selected_model_id.as_str(),
            provider_id: input.selected_provider_id.as_str(),
        }),
    }
}

pub fn normalize_code_runner_source(source: &str) -> String {
    let trimmed = source.trim_start();
    if trimmed.starts_with("export function run") {
        source.replacen("export function run", "function run", 1)
    } else {
        source.to_string()
    }
}

pub fn parse_code_runner_output(node_id: &str, json: &str) -> Result<RuntimeExecuteOutput, String> {
    let output = serde_json::from_str::<CodeRunnerOutput>(json).map_err(|error| {
        format!("code_runner node '{node_id}' returned invalid output JSON: {error}")
    })?;
    Ok(RuntimeExecuteOutput {
        context_ops: output
            .context_patch
            .into_iter()
            .map(|op| match op {
                CodeRunnerContextPatchOp::Set { key, value } => {
                    RuntimeContextPatchOp::Set { key, value }
                }
                CodeRunnerContextPatchOp::Remove { key } => RuntimeContextPatchOp::Remove { key },
            })
            .collect(),
        header_ops: output
            .header_ops
            .into_iter()
            .map(|op| match op {
                CodeRunnerHeaderOp::Set { name, value } => RuntimeHeaderOp::Set { name, value },
                CodeRunnerHeaderOp::Append { name, value } => RuntimeHeaderOp::Append { name, value },
                CodeRunnerHeaderOp::Remove { name } => RuntimeHeaderOp::Remove { name },
            })
            .collect(),
        path_rewrite: output.path_rewrite,
        next_port: output.next_port,
        logs: output.logs.into_iter().map(|log| RuntimeLogEntry {
            level: match log.level {
                CodeRunnerLogLevel::Debug => RuntimeLogLevel::Debug,
                CodeRunnerLogLevel::Info => RuntimeLogLevel::Info,
                CodeRunnerLogLevel::Warn => RuntimeLogLevel::Warn,
                CodeRunnerLogLevel::Error => RuntimeLogLevel::Error,
            },
            message: log.message,
        }).collect(),
    })
}

pub fn runtime_capability_from_manifest(capability: &ManifestCapability) -> RuntimeCapabilityKind {
    match capability {
        ManifestCapability::Log => RuntimeCapabilityKind::Log,
        ManifestCapability::Fs => RuntimeCapabilityKind::Fs,
        ManifestCapability::Network => RuntimeCapabilityKind::Network,
    }
}

pub fn wasm_capability_from_manifest(capability: &ManifestCapability) -> WasmCapability {
    match capability {
        ManifestCapability::Log => WasmCapability::Log,
        ManifestCapability::Fs => WasmCapability::Fs,
        ManifestCapability::Network => WasmCapability::Network,
    }
}

pub fn runtime_capability_from_wasm(capability: WasmCapability) -> RuntimeCapabilityKind {
    match capability {
        WasmCapability::Log => RuntimeCapabilityKind::Log,
        WasmCapability::Fs => RuntimeCapabilityKind::Fs,
        WasmCapability::Network => RuntimeCapabilityKind::Network,
    }
}

pub fn runtime_capability_name(kind: &RuntimeCapabilityKind) -> &'static str {
    match kind {
        RuntimeCapabilityKind::Log => "log",
        RuntimeCapabilityKind::Fs => "fs",
        RuntimeCapabilityKind::Network => "network",
    }
}

pub fn capability_scope(kind: WasmCapability, node_config: &WasmPluginNodeConfig) -> Option<String> {
    match kind {
        WasmCapability::Log => None,
        WasmCapability::Fs => {
            let reads = node_config.read_dirs.join(",");
            let writes = node_config.write_dirs.join(",");
            Some(format!("read={reads};write={writes}"))
        }
        WasmCapability::Network => Some(node_config.allowed_hosts.join(",")),
    }
}

pub fn toml_value_to_json(value: &toml::Value) -> String {
    match value {
        toml::Value::String(value) => format!("\"{}\"", escape_json_string(value)),
        toml::Value::Integer(value) => value.to_string(),
        toml::Value::Float(value) => value.to_string(),
        toml::Value::Boolean(value) => value.to_string(),
        toml::Value::Datetime(value) => format!("\"{}\"", escape_json_string(&value.to_string())),
        toml::Value::Array(items) => format!("[{}]", items.iter().map(toml_value_to_json).collect::<Vec<_>>().join(",")),
        toml::Value::Table(table) => format!("{{{}}}", table.iter().map(|(key, value)| format!("\"{}\":{}", escape_json_string(key), toml_value_to_json(value))).collect::<Vec<_>>().join(",")),
    }
}

pub fn escape_json_string(value: &str) -> String {
    value
        .chars()
        .flat_map(|ch| match ch {
            '"' => ['\\', '"'].into_iter().collect::<Vec<_>>(),
            '\\' => ['\\', '\\'].into_iter().collect(),
            '\n' => ['\\', 'n'].into_iter().collect(),
            '\r' => ['\\', 'r'].into_iter().collect(),
            '\t' => ['\\', 't'].into_iter().collect(),
            _ => [ch].into_iter().collect(),
        })
        .collect()
}
