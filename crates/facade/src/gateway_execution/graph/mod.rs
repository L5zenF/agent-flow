mod nodes;
mod state;

use axum::http::{HeaderMap, Method, Uri};
use infrastructure::plugin_registry::{LoadedPlugin, PluginRegistry};

use crate::config::{
    CodeRunnerNodeConfig, GatewayConfig, ModelConfig, ProviderConfig, RuleGraphConfig,
    WasmPluginNodeConfig,
};

use self::state::GraphExecutionState;
use super::RequestResolution;

#[allow(clippy::too_many_arguments)]
pub trait GraphNodeExecutor: Sync {
    fn execute_wasm_runtime_node(
        &self,
        node_id: &str,
        node_config: &WasmPluginNodeConfig,
        plugin_registry: &PluginRegistry,
        method: &str,
        headers: &HeaderMap,
        selected_provider: Option<&ProviderConfig>,
        selected_model: Option<&ModelConfig>,
        resolved_path: &mut String,
        workflow_context: &mut std::collections::HashMap<String, String>,
        outgoing_headers: &mut std::collections::HashMap<String, Vec<String>>,
    ) -> Result<Option<String>, String>;

    #[allow(clippy::too_many_arguments)]
    fn execute_code_runner_node(
        &self,
        plugin_registry: &PluginRegistry,
        node_id: &str,
        node_config: &CodeRunnerNodeConfig,
        method: &str,
        headers: &HeaderMap,
        selected_provider: Option<&ProviderConfig>,
        selected_model: Option<&ModelConfig>,
        resolved_path: &mut String,
        workflow_context: &mut std::collections::HashMap<String, String>,
        outgoing_headers: &mut std::collections::HashMap<String, Vec<String>>,
    ) -> Result<Option<String>, String>;
}

pub fn execute_rule_graph<'cfg>(
    config: &'cfg GatewayConfig,
    plugin_registry: &PluginRegistry,
    executor: &dyn GraphNodeExecutor,
    graph: &RuleGraphConfig,
    method: &Method,
    uri: &Uri,
    headers: &HeaderMap,
) -> Result<RequestResolution<'cfg>, String> {
    GraphExecutionState::new(
        config,
        plugin_registry,
        executor,
        graph,
        method,
        uri,
        headers,
    )?
    .run()
}

fn validate_plugin_port(plugin: &LoadedPlugin, port: &str, node_id: &str) -> Result<(), String> {
    if plugin
        .manifest()
        .supported_output_ports
        .iter()
        .any(|item| item == port)
    {
        return Ok(());
    }

    Err(format!(
        "plugin '{}' returned unknown port '{}' for node '{}'",
        plugin.plugin_id(),
        port,
        node_id
    ))
}
