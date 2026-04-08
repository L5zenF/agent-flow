mod input;
mod output;
mod wit_codec;

use std::collections::HashMap;

use axum::http::HeaderMap;
use infrastructure::plugin_registry::{LoadedPlugin, ManifestCapability, PluginRegistry};
use infrastructure::plugin_runtime_contract::{
    RuntimeCapabilityKind, RuntimeExecuteInput, RuntimeExecuteOutput,
};

use super::PluginNodeRuntime;
use super::component_runtime::log_runtime_output;
use super::exports::proxy_tools::proxy_node_plugin::node_plugin::{
    CapabilityKind, ExecuteInput, ExecuteOutput, LogLevel,
};
use crate::config::{ModelConfig, ProviderConfig, WasmCapability, WasmPluginNodeConfig};

use self::input::build_runtime_execute_input;
use self::output::apply_runtime_output;

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

pub(super) fn to_wit_execute_input(input: &RuntimeExecuteInput) -> ExecuteInput {
    wit_codec::to_wit_execute_input(input)
}

pub(super) fn from_wit_execute_output(output: ExecuteOutput) -> RuntimeExecuteOutput {
    wit_codec::from_wit_execute_output(output)
}

pub(super) fn runtime_capability_from_manifest(
    capability: &ManifestCapability,
) -> RuntimeCapabilityKind {
    match capability {
        ManifestCapability::Log => RuntimeCapabilityKind::Log,
        ManifestCapability::Fs => RuntimeCapabilityKind::Fs,
        ManifestCapability::Network => RuntimeCapabilityKind::Network,
    }
}

pub(super) fn wasm_capability_from_manifest(capability: &ManifestCapability) -> WasmCapability {
    match capability {
        ManifestCapability::Log => WasmCapability::Log,
        ManifestCapability::Fs => WasmCapability::Fs,
        ManifestCapability::Network => WasmCapability::Network,
    }
}

pub(super) fn runtime_capability_from_wasm(capability: WasmCapability) -> RuntimeCapabilityKind {
    match capability {
        WasmCapability::Log => RuntimeCapabilityKind::Log,
        WasmCapability::Fs => RuntimeCapabilityKind::Fs,
        WasmCapability::Network => RuntimeCapabilityKind::Network,
    }
}

pub(super) fn runtime_capability_name(kind: &RuntimeCapabilityKind) -> &'static str {
    match kind {
        RuntimeCapabilityKind::Log => "log",
        RuntimeCapabilityKind::Fs => "fs",
        RuntimeCapabilityKind::Network => "network",
    }
}

pub(super) fn capability_scope(
    kind: WasmCapability,
    node_config: &WasmPluginNodeConfig,
) -> Option<String> {
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

pub(super) fn to_wit_capability_kind(kind: &RuntimeCapabilityKind) -> CapabilityKind {
    match kind {
        RuntimeCapabilityKind::Log => CapabilityKind::Log,
        RuntimeCapabilityKind::Fs => CapabilityKind::Fs,
        RuntimeCapabilityKind::Network => CapabilityKind::Network,
    }
}

pub(super) fn from_wit_log_level(
    level: LogLevel,
) -> infrastructure::plugin_runtime_contract::RuntimeLogLevel {
    match level {
        LogLevel::Debug => infrastructure::plugin_runtime_contract::RuntimeLogLevel::Debug,
        LogLevel::Info => infrastructure::plugin_runtime_contract::RuntimeLogLevel::Info,
        LogLevel::Warn => infrastructure::plugin_runtime_contract::RuntimeLogLevel::Warn,
        LogLevel::Error => infrastructure::plugin_runtime_contract::RuntimeLogLevel::Error,
    }
}

pub(super) fn log_output(node_id: &str, plugin: &LoadedPlugin, output: &RuntimeExecuteOutput) {
    log_runtime_output(node_id, Some(plugin), output);
}
