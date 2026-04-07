mod code_runner;
mod contracts;
mod wasi;
mod component_runtime;

use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::mpsc::SyncSender;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use axum::http::HeaderMap;
use infrastructure::plugin_runtime_contract::{RuntimeExecuteInput, RuntimeExecuteOutput};
use infrastructure::plugin_registry::{LoadedPlugin, PluginRegistry};
use ::wasmtime::component::ResourceTable;
use ::wasmtime::{Engine, StoreLimits};
use wasmtime_wasi::p1::WasiP1Ctx;
use wasmtime_wasi::{DirPerms, FilePerms, WasiCtx, WasiCtxView, WasiView};

use crate::config::{CodeRunnerNodeConfig, ModelConfig, ProviderConfig, WasmPluginNodeConfig};
use crate::gateway_execution::GraphNodeExecutor;
use self::exports::proxy_tools::proxy_node_plugin::node_plugin::ExecuteError;

#[cfg(test)]
pub(crate) use wasi::{
    plugin_workspace_root, resolve_plugin_network_policy, resolve_plugin_preopens,
    socket_addr_allowed,
};
pub(crate) use component_runtime::WASMTIME_PLUGIN_RUNTIME;

::wasmtime::component::bindgen!({
    path: "../../wit",
    world: "proxy-node-plugin",
    ownership: Owning,
});

pub(crate) trait PluginNodeRuntime: Sync {
    fn execute(
        &self,
        plugin: &LoadedPlugin,
        timeout_ms: u64,
        fuel: Option<u64>,
        max_memory_bytes: u64,
        node_config: &WasmPluginNodeConfig,
        input: &RuntimeExecuteInput,
    ) -> Result<RuntimeExecuteOutput, String>;
}

pub(crate) struct GatewayGraphExecutor<'a> {
    pub(crate) runtime: &'a dyn PluginNodeRuntime,
}

pub(crate) struct PluginStoreData {
    pub(crate) limits: StoreLimits,
    pub(crate) table: ResourceTable,
    pub(crate) wasi: WasiCtx,
}

pub(crate) struct CorePluginStoreData {
    pub(crate) limits: StoreLimits,
    pub(crate) wasi: WasiP1Ctx,
}

#[derive(Debug, Clone)]
pub(crate) struct PluginPreopenDir {
    pub(crate) host_path: PathBuf,
    pub(crate) guest_path: String,
    pub(crate) dir_perms: DirPerms,
    pub(crate) file_perms: FilePerms,
}

#[derive(Clone)]
pub(crate) struct PluginNetworkPolicy {
    pub(crate) allowed_addrs: Arc<HashSet<SocketAddr>>,
    pub(crate) allow_ip_name_lookup: bool,
}

pub(crate) struct TimeoutEpochGuard {
    pub(crate) cancel: Option<SyncSender<()>>,
    pub(crate) handle: Option<JoinHandle<()>>,
}

impl TimeoutEpochGuard {
    pub(crate) fn start(engine: Engine, timeout: Duration) -> Self {
        let (cancel, receiver) = std::sync::mpsc::sync_channel::<()>(1);
        let handle = std::thread::spawn(move || {
            if receiver.recv_timeout(timeout).is_err() {
                engine.increment_epoch();
            }
        });

        Self {
            cancel: Some(cancel),
            handle: Some(handle),
        }
    }
}

pub(super) fn describe_execute_error(error: &ExecuteError) -> &str {
    match error {
        ExecuteError::Denied(message)
        | ExecuteError::InvalidInput(message)
        | ExecuteError::Failed(message) => message.as_str(),
    }
}

impl Drop for TimeoutEpochGuard {
    fn drop(&mut self) {
        if let Some(cancel) = self.cancel.take() {
            let _ = cancel.send(());
        }
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl WasiView for PluginStoreData {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl GraphNodeExecutor for GatewayGraphExecutor<'_> {
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
        workflow_context: &mut HashMap<String, String>,
        outgoing_headers: &mut HashMap<String, Vec<String>>,
    ) -> Result<Option<String>, String> {
        contracts::execute_wasm_runtime_node(
            node_id,
            node_config,
            plugin_registry,
            self.runtime,
            method,
            headers,
            selected_provider,
            selected_model,
            resolved_path,
            workflow_context,
            outgoing_headers,
        )
    }

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
        workflow_context: &mut HashMap<String, String>,
        outgoing_headers: &mut HashMap<String, Vec<String>>,
    ) -> Result<Option<String>, String> {
        code_runner::execute_code_runner_node(
            plugin_registry,
            node_id,
            node_config,
            method,
            headers,
            selected_provider,
            selected_model,
            resolved_path,
            workflow_context,
            outgoing_headers,
        )
    }
}
