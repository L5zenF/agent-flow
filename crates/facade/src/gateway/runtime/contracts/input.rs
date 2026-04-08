use std::collections::HashSet;

use infrastructure::plugin_registry::LoadedPlugin;
use infrastructure::plugin_runtime_contract::{
    RuntimeCapabilityDeclaration, RuntimeCapabilityGrant, RuntimeExecuteInput, RuntimeNodeConfig,
    RuntimePluginManifest, RuntimeRequestHeader,
};

use crate::config::{ModelConfig, ProviderConfig, WasmPluginNodeConfig};

use super::{
    capability_scope, runtime_capability_from_manifest, runtime_capability_from_wasm,
    runtime_capability_name, wasm_capability_from_manifest,
};

#[allow(clippy::too_many_arguments)]
pub(super) fn build_runtime_execute_input(
    plugin: &LoadedPlugin,
    node_id: &str,
    node_config: &WasmPluginNodeConfig,
    method: &str,
    current_path: &str,
    request_headers: &[RuntimeRequestHeader],
    workflow_context: &std::collections::HashMap<String, String>,
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
