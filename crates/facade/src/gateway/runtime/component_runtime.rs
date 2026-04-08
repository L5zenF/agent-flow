use ::wasmtime::component::Linker as ComponentLinker;
use ::wasmtime::{Store, StoreLimitsBuilder};
use infrastructure::plugin_registry::LoadedPlugin;
use infrastructure::plugin_runtime_contract::{
    RuntimeExecuteInput, RuntimeExecuteOutput, RuntimeLogLevel,
};
use tracing::{error, info, warn};

use super::contracts::{from_wit_execute_output, to_wit_execute_input};
use super::wasi::build_plugin_wasi_ctx;
use super::{
    PluginNodeRuntime, PluginStoreData, ProxyNodePlugin, TimeoutEpochGuard, describe_execute_error,
};
use crate::config::WasmPluginNodeConfig;

pub(crate) struct WasmtimePluginRuntime;

pub(crate) static WASMTIME_PLUGIN_RUNTIME: WasmtimePluginRuntime = WasmtimePluginRuntime;

impl PluginNodeRuntime for WasmtimePluginRuntime {
    fn execute(
        &self,
        plugin: &LoadedPlugin,
        timeout_ms: u64,
        fuel: Option<u64>,
        max_memory_bytes: u64,
        node_config: &WasmPluginNodeConfig,
        input: &RuntimeExecuteInput,
    ) -> Result<RuntimeExecuteOutput, String> {
        let mut linker = ComponentLinker::new(plugin.component().engine());
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker).map_err(|error| {
            format!(
                "failed to wire WASI imports for plugin '{}': {error}",
                plugin.plugin_id()
            )
        })?;
        let wasi = build_plugin_wasi_ctx(plugin, node_config)?;
        let limits = StoreLimitsBuilder::new()
            .memory_size(max_memory_bytes.try_into().unwrap_or(usize::MAX))
            .trap_on_grow_failure(true)
            .build();
        let mut store = Store::new(
            plugin.component().engine(),
            PluginStoreData {
                limits,
                table: wasmtime::component::ResourceTable::new(),
                wasi,
            },
        );
        store.limiter(|state| &mut state.limits);
        store
            .set_fuel(fuel.unwrap_or(u64::MAX))
            .map_err(|error| format!("failed to set plugin fuel budget: {error}"))?;
        store.set_epoch_deadline(1);
        store.epoch_deadline_trap();

        let bindings = ProxyNodePlugin::instantiate(&mut store, plugin.component(), &linker)
            .map_err(|error| {
                format!(
                    "failed to instantiate plugin '{}': {error}",
                    plugin.plugin_id()
                )
            })?;
        let _timeout_guard = TimeoutEpochGuard::start(
            plugin.component().engine().clone(),
            std::time::Duration::from_millis(timeout_ms),
        );
        let output = bindings
            .interface0
            .call_execute(&mut store, &to_wit_execute_input(input))
            .map_err(|error| {
                if error.to_string().contains("epoch deadline") {
                    format!(
                        "plugin '{}' exceeded timeout of {timeout_ms}ms",
                        plugin.plugin_id()
                    )
                } else {
                    format!("plugin '{}' execution failed: {error}", plugin.plugin_id())
                }
            })?
            .map_err(|error| {
                format!(
                    "plugin '{}' returned an execution error: {}",
                    plugin.plugin_id(),
                    describe_execute_error(&error)
                )
            })?;

        Ok(from_wit_execute_output(output))
    }
}

pub(crate) fn log_runtime_output(
    node_id: &str,
    plugin: Option<&LoadedPlugin>,
    output: &RuntimeExecuteOutput,
) {
    for log in &output.logs {
        match (plugin, &log.level) {
            (Some(plugin), RuntimeLogLevel::Debug) => {
                info!(plugin_id = %plugin.plugin_id(), node_id = %node_id, wasm_log = %log.message, "[wasm][debug]")
            }
            (Some(plugin), RuntimeLogLevel::Info) => {
                info!(plugin_id = %plugin.plugin_id(), node_id = %node_id, wasm_log = %log.message, "[wasm][info]")
            }
            (Some(plugin), RuntimeLogLevel::Warn) => {
                warn!(plugin_id = %plugin.plugin_id(), node_id = %node_id, wasm_log = %log.message, "[wasm][warn]")
            }
            (Some(plugin), RuntimeLogLevel::Error) => {
                error!(plugin_id = %plugin.plugin_id(), node_id = %node_id, wasm_log = %log.message, "[wasm][error]")
            }
            (None, RuntimeLogLevel::Debug) => {
                info!(node_id = %node_id, code_runner_log = %log.message, "[code-runner][debug]")
            }
            (None, RuntimeLogLevel::Info) => {
                info!(node_id = %node_id, code_runner_log = %log.message, "[code-runner][info]")
            }
            (None, RuntimeLogLevel::Warn) => {
                warn!(node_id = %node_id, code_runner_log = %log.message, "[code-runner][warn]")
            }
            (None, RuntimeLogLevel::Error) => {
                error!(node_id = %node_id, code_runner_log = %log.message, "[code-runner][error]")
            }
        }
    }
}
