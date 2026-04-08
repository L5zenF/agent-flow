mod core_runtime;
mod io;

use std::collections::HashMap;

use axum::http::HeaderMap;
use infrastructure::plugin_registry::{PluginRegistry, PluginRuntimeKind};
use infrastructure::plugin_runtime_contract::RuntimeExecuteInput;

use crate::config::{CodeRunnerNodeConfig, ModelConfig, ProviderConfig};

use self::core_runtime::execute_core_code_runner_plugin;
use self::io::{apply_output, build_runtime_input};

const CODE_RUNNER_PLUGIN_ID: &str = "js-code-runner";

#[allow(clippy::too_many_arguments)]
pub(crate) fn execute_code_runner_node(
    plugin_registry: &PluginRegistry,
    node_id: &str,
    node_config: &CodeRunnerNodeConfig,
    method: &str,
    headers: &HeaderMap,
    selected_provider: Option<&ProviderConfig>,
    selected_model: Option<&ModelConfig>,
    resolved_path: &mut String,
    workflow_context: &mut HashMap<String, String>,
    outgoing_headers: &mut HashMap<String, Vec<String>>,
) -> Result<Option<String>, String> {
    let plugin = plugin_registry.get(CODE_RUNNER_PLUGIN_ID).ok_or_else(|| {
        format!(
            "code_runner plugin '{}' is not registered",
            CODE_RUNNER_PLUGIN_ID
        )
    })?;
    if plugin.runtime_kind() != PluginRuntimeKind::Core {
        return Err(format!(
            "code_runner plugin '{}' must use runtime = \"core\"",
            CODE_RUNNER_PLUGIN_ID
        ));
    }

    let runtime_input = build_runtime_input(
        node_id,
        method,
        resolved_path,
        headers,
        workflow_context,
        outgoing_headers,
        selected_provider,
        selected_model,
    );
    let output = execute_core_code_runner_plugin(plugin, node_id, node_config, &runtime_input)?;
    apply_output(
        node_id,
        &output,
        workflow_context,
        outgoing_headers,
        resolved_path,
    );
    Ok(output.next_port)
}

fn empty_runtime_execute_input(node_id: &str) -> RuntimeExecuteInput {
    RuntimeExecuteInput {
        request_method: String::new(),
        current_path: String::new(),
        request_headers: Vec::new(),
        workflow_context: Vec::new(),
        selected_provider_id: String::new(),
        selected_model_id: String::new(),
        node_config: infrastructure::plugin_runtime_contract::RuntimeNodeConfig {
            manifest: infrastructure::plugin_runtime_contract::RuntimePluginManifest {
                id: node_id.to_string(),
                name: "Code Runner".to_string(),
                version: String::new(),
                description: String::new(),
                supported_output_ports: Vec::new(),
                default_config_schema_hints_json: None,
                capabilities: Vec::new(),
            },
            grants: Vec::new(),
            config_json: None,
        },
    }
}
