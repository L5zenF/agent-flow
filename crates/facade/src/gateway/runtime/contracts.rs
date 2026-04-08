use std::collections::{HashMap, HashSet};

use axum::http::HeaderMap;
use infrastructure::plugin_registry::{LoadedPlugin, ManifestCapability, PluginRegistry};
use infrastructure::plugin_runtime_contract::{
    RuntimeCapabilityDeclaration, RuntimeCapabilityGrant, RuntimeCapabilityKind,
    RuntimeContextPatchOp, RuntimeExecuteInput, RuntimeExecuteOutput, RuntimeHeaderOp,
    RuntimeLogEntry, RuntimeLogLevel, RuntimeNodeConfig, RuntimePluginManifest,
    RuntimeRequestHeader,
};

use super::PluginNodeRuntime;
use super::component_runtime::log_runtime_output;
use super::exports::proxy_tools::proxy_node_plugin::node_plugin::{
    CapabilityDeclaration, CapabilityGrant, CapabilityKind, ContextEntry, ContextPatchOp,
    ExecuteInput, ExecuteOutput, HeaderOp, JsonDocument, LogLevel, NodeConfig, PluginManifest,
    RequestHeader,
};
use crate::config::{ModelConfig, ProviderConfig, WasmCapability, WasmPluginNodeConfig};

#[allow(clippy::too_many_arguments)]
pub(crate) fn execute_wasm_runtime_node(
    node_id: &str,
    node_config: &WasmPluginNodeConfig,
    plugin_registry: &PluginRegistry,
    runtime: &dyn PluginNodeRuntime,
    method: &str,
    headers: &HeaderMap,
    selected_provider: Option<&ProviderConfig>,
    selected_model: Option<&ModelConfig>,
    resolved_path: &mut String,
    workflow_context: &mut HashMap<String, String>,
    outgoing_headers: &mut HashMap<String, Vec<String>>,
) -> Result<Option<String>, String> {
    let plugin = plugin_registry
        .get(node_config.plugin_id.as_str())
        .ok_or_else(|| {
            format!(
                "rule_graph node '{}' references unknown plugin '{}'",
                node_id, node_config.plugin_id
            )
        })?;
    let runtime_input = build_runtime_execute_input(
        plugin,
        node_id,
        node_config,
        method,
        resolved_path.as_str(),
        &super::wasi::current_request_headers(headers, outgoing_headers),
        workflow_context,
        selected_provider,
        selected_model,
    )?;
    let runtime_output = runtime.execute(
        plugin,
        node_config.timeout_ms,
        node_config.fuel,
        node_config.max_memory_bytes,
        node_config,
        &runtime_input,
    )?;

    apply_runtime_output(
        node_id,
        plugin,
        &runtime_output,
        workflow_context,
        outgoing_headers,
        resolved_path,
    );
    Ok(runtime_output.next_port)
}

fn build_runtime_execute_input(
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

fn apply_runtime_output(
    node_id: &str,
    plugin: &LoadedPlugin,
    output: &RuntimeExecuteOutput,
    workflow_context: &mut HashMap<String, String>,
    outgoing_headers: &mut HashMap<String, Vec<String>>,
    resolved_path: &mut String,
) {
    log_runtime_output(node_id, Some(plugin), output);
    for op in &output.context_ops {
        match op {
            RuntimeContextPatchOp::Set { key, value } => {
                workflow_context.insert(key.clone(), value.clone());
            }
            RuntimeContextPatchOp::Remove { key } => {
                workflow_context.remove(key);
            }
        }
    }
    for op in &output.header_ops {
        match op {
            RuntimeHeaderOp::Set { name, value } => {
                outgoing_headers.insert(name.to_ascii_lowercase(), vec![value.clone()]);
            }
            RuntimeHeaderOp::Append { name, value } => {
                outgoing_headers
                    .entry(name.to_ascii_lowercase())
                    .or_default()
                    .push(value.clone());
            }
            RuntimeHeaderOp::Remove { name } => {
                outgoing_headers.remove(&name.to_ascii_lowercase());
            }
        }
    }
    if let Some(path_rewrite) = &output.path_rewrite {
        *resolved_path = path_rewrite.clone();
    }
}

pub(super) fn to_wit_execute_input(input: &RuntimeExecuteInput) -> ExecuteInput {
    ExecuteInput {
        request_method: input.request_method.clone(),
        current_path: input.current_path.clone(),
        request_headers: input
            .request_headers
            .iter()
            .map(|header| RequestHeader {
                name: header.name.clone(),
                value: header.value.clone(),
            })
            .collect(),
        workflow_context: input
            .workflow_context
            .iter()
            .map(|(key, value)| ContextEntry {
                key: key.clone(),
                value: value.clone(),
            })
            .collect(),
        selected_provider_id: input.selected_provider_id.clone(),
        selected_model_id: input.selected_model_id.clone(),
        node_config: NodeConfig {
            manifest: PluginManifest {
                id: input.node_config.manifest.id.clone(),
                name: input.node_config.manifest.name.clone(),
                version: input.node_config.manifest.version.clone(),
                description: input.node_config.manifest.description.clone(),
                supported_output_ports: input.node_config.manifest.supported_output_ports.clone(),
                default_config_schema_hints: input
                    .node_config
                    .manifest
                    .default_config_schema_hints_json
                    .as_ref()
                    .map(|json| JsonDocument { json: json.clone() }),
                capabilities: input
                    .node_config
                    .manifest
                    .capabilities
                    .iter()
                    .map(|capability| CapabilityDeclaration {
                        kind: to_wit_capability_kind(&capability.kind),
                        required: capability.required,
                        scope: capability.scope.clone(),
                        description: capability.description.clone(),
                    })
                    .collect(),
            },
            grants: input
                .node_config
                .grants
                .iter()
                .map(|grant| CapabilityGrant {
                    kind: to_wit_capability_kind(&grant.kind),
                    allowed: grant.allowed,
                    scope: grant.scope.clone(),
                })
                .collect(),
            config: input
                .node_config
                .config_json
                .as_ref()
                .map(|json| JsonDocument { json: json.clone() }),
        },
    }
}

pub(super) fn from_wit_execute_output(output: ExecuteOutput) -> RuntimeExecuteOutput {
    RuntimeExecuteOutput {
        context_ops: output
            .context_patch
            .map(|patch| {
                patch
                    .ops
                    .into_iter()
                    .map(|op| match op {
                        ContextPatchOp::Set(entry) => RuntimeContextPatchOp::Set {
                            key: entry.key,
                            value: entry.value,
                        },
                        ContextPatchOp::Remove(key) => RuntimeContextPatchOp::Remove { key },
                    })
                    .collect()
            })
            .unwrap_or_default(),
        header_ops: output
            .header_ops
            .into_iter()
            .map(|op| match op {
                HeaderOp::Set(header) => RuntimeHeaderOp::Set {
                    name: header.name,
                    value: header.value,
                },
                HeaderOp::Append(header) => RuntimeHeaderOp::Append {
                    name: header.name,
                    value: header.value,
                },
                HeaderOp::Remove(name) => RuntimeHeaderOp::Remove { name },
            })
            .collect(),
        path_rewrite: output.path_rewrite.map(|path| path.path),
        next_port: output.next_port.map(|port| port.port),
        logs: output
            .logs
            .into_iter()
            .map(|log| RuntimeLogEntry {
                level: from_wit_log_level(log.level),
                message: log.message,
            })
            .collect(),
    }
}

fn runtime_capability_from_manifest(capability: &ManifestCapability) -> RuntimeCapabilityKind {
    match capability {
        ManifestCapability::Log => RuntimeCapabilityKind::Log,
        ManifestCapability::Fs => RuntimeCapabilityKind::Fs,
        ManifestCapability::Network => RuntimeCapabilityKind::Network,
    }
}

fn wasm_capability_from_manifest(capability: &ManifestCapability) -> WasmCapability {
    match capability {
        ManifestCapability::Log => WasmCapability::Log,
        ManifestCapability::Fs => WasmCapability::Fs,
        ManifestCapability::Network => WasmCapability::Network,
    }
}

fn runtime_capability_from_wasm(capability: WasmCapability) -> RuntimeCapabilityKind {
    match capability {
        WasmCapability::Log => RuntimeCapabilityKind::Log,
        WasmCapability::Fs => RuntimeCapabilityKind::Fs,
        WasmCapability::Network => RuntimeCapabilityKind::Network,
    }
}

fn runtime_capability_name(kind: &RuntimeCapabilityKind) -> &'static str {
    match kind {
        RuntimeCapabilityKind::Log => "log",
        RuntimeCapabilityKind::Fs => "fs",
        RuntimeCapabilityKind::Network => "network",
    }
}

fn capability_scope(kind: WasmCapability, node_config: &WasmPluginNodeConfig) -> Option<String> {
    match kind {
        WasmCapability::Log => None,
        WasmCapability::Fs => Some(format!(
            "read={};write={}",
            node_config.read_dirs.join(","),
            node_config.write_dirs.join(",")
        )),
        WasmCapability::Network => Some(node_config.allowed_hosts.join(",")),
    }
}

fn toml_value_to_json(value: &toml::Value) -> String {
    match value {
        toml::Value::String(value) => format!("\"{}\"", escape_json_string(value)),
        toml::Value::Integer(value) => value.to_string(),
        toml::Value::Float(value) => value.to_string(),
        toml::Value::Boolean(value) => value.to_string(),
        toml::Value::Datetime(value) => format!("\"{}\"", escape_json_string(&value.to_string())),
        toml::Value::Array(items) => format!(
            "[{}]",
            items
                .iter()
                .map(toml_value_to_json)
                .collect::<Vec<_>>()
                .join(",")
        ),
        toml::Value::Table(table) => format!(
            "{{{}}}",
            table
                .iter()
                .map(|(key, value)| format!(
                    "\"{}\":{}",
                    escape_json_string(key),
                    toml_value_to_json(value)
                ))
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}

fn escape_json_string(value: &str) -> String {
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

fn to_wit_capability_kind(kind: &RuntimeCapabilityKind) -> CapabilityKind {
    match kind {
        RuntimeCapabilityKind::Log => CapabilityKind::Log,
        RuntimeCapabilityKind::Fs => CapabilityKind::Fs,
        RuntimeCapabilityKind::Network => CapabilityKind::Network,
    }
}

fn from_wit_log_level(level: LogLevel) -> RuntimeLogLevel {
    match level {
        LogLevel::Debug => RuntimeLogLevel::Debug,
        LogLevel::Info => RuntimeLogLevel::Info,
        LogLevel::Warn => RuntimeLogLevel::Warn,
        LogLevel::Error => RuntimeLogLevel::Error,
    }
}
