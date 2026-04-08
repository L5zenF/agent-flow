use std::collections::HashMap;

use ::wasmtime::Linker as CoreLinker;
use ::wasmtime::{Memory, Store, StoreLimitsBuilder};
use axum::http::HeaderMap;
use infrastructure::plugin_registry::{PluginRegistry, PluginRuntimeKind};
use infrastructure::plugin_runtime_contract::{
    CodeRunnerContextPatchOp, CodeRunnerHeaderOp, CodeRunnerInput, CodeRunnerLogLevel,
    CodeRunnerModel, CodeRunnerOutput, CodeRunnerProvider, CoreCodeRunnerRequest,
    RuntimeContextPatchOp, RuntimeExecuteInput, RuntimeExecuteOutput, RuntimeHeaderOp,
    RuntimeLogEntry, RuntimeLogLevel, RuntimeNodeConfig, RuntimePluginManifest,
};
use serde::Deserialize;
use wasmtime_wasi::WasiCtxBuilder;

use super::component_runtime::log_runtime_output;
use super::{CorePluginStoreData, TimeoutEpochGuard};
use crate::config::{CodeRunnerNodeConfig, ModelConfig, ProviderConfig};

const CODE_RUNNER_PLUGIN_ID: &str = "js-code-runner";

#[derive(Debug, Deserialize)]
struct CoreCodeRunnerResponse {
    ok: bool,
    #[serde(default)]
    json: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

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
    let runtime_input = RuntimeExecuteInput {
        request_method: method.to_string(),
        current_path: resolved_path.to_string(),
        request_headers: super::wasi::current_request_headers(headers, outgoing_headers),
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
    };

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

fn execute_core_code_runner_plugin(
    plugin: &infrastructure::plugin_registry::LoadedPlugin,
    node_id: &str,
    node_config: &CodeRunnerNodeConfig,
    input: &RuntimeExecuteInput,
) -> Result<RuntimeExecuteOutput, String> {
    let mut linker = CoreLinker::new(plugin.module().engine());
    wasmtime_wasi::p1::add_to_linker_sync(&mut linker, |state: &mut CorePluginStoreData| {
        &mut state.wasi
    })
    .map_err(|error| {
        format!(
            "failed to wire WASI imports for code_runner plugin '{}': {error}",
            plugin.plugin_id()
        )
    })?;
    let limits = StoreLimitsBuilder::new()
        .memory_size(
            node_config
                .max_memory_bytes
                .try_into()
                .unwrap_or(usize::MAX),
        )
        .trap_on_grow_failure(true)
        .build();
    let mut store = Store::new(
        plugin.module().engine(),
        CorePluginStoreData {
            limits,
            wasi: WasiCtxBuilder::new().build_p1(),
        },
    );
    store.limiter(|state| &mut state.limits);
    store
        .set_fuel(u64::MAX)
        .map_err(|error| format!("failed to set code_runner fuel budget: {error}"))?;
    store.set_epoch_deadline(1);
    store.epoch_deadline_trap();

    let instance = linker
        .instantiate(&mut store, plugin.module())
        .map_err(|error| {
            format!(
                "failed to instantiate code_runner plugin '{}': {error}",
                plugin.plugin_id()
            )
        })?;
    let memory = instance.get_memory(&mut store, "memory").ok_or_else(|| {
        format!(
            "code_runner plugin '{}' is missing exported memory",
            plugin.plugin_id()
        )
    })?;
    let alloc = instance
        .get_typed_func::<u32, u32>(&mut store, "alloc")
        .map_err(|error| {
            format!(
                "code_runner plugin '{}' is missing alloc export: {error}",
                plugin.plugin_id()
            )
        })?;
    let dealloc = instance
        .get_typed_func::<(u32, u32), ()>(&mut store, "dealloc")
        .map_err(|error| {
            format!(
                "code_runner plugin '{}' is missing dealloc export: {error}",
                plugin.plugin_id()
            )
        })?;
    let run_json = instance
        .get_typed_func::<(u32, u32), u64>(&mut store, "run_json")
        .map_err(|error| {
            format!(
                "code_runner plugin '{}' is missing run_json export: {error}",
                plugin.plugin_id()
            )
        })?;

    let request_json = serde_json::to_vec(&CoreCodeRunnerRequest {
        code: normalize_code_runner_source(node_config.code.as_str()),
        input: build_code_runner_input(input),
    })
    .map_err(|error| format!("code_runner node '{node_id}' failed to serialize input: {error}"))?;
    let _timeout_guard = TimeoutEpochGuard::start(
        plugin.module().engine().clone(),
        std::time::Duration::from_millis(node_config.timeout_ms),
    );
    let request_ptr = alloc
        .call(&mut store, request_json.len() as u32)
        .map_err(|error| {
            format!(
                "code_runner plugin '{}' failed to allocate input buffer: {error}",
                plugin.plugin_id()
            )
        })?;
    memory
        .write(&mut store, request_ptr as usize, &request_json)
        .map_err(|error| {
            format!(
                "code_runner plugin '{}' failed to write input buffer: {error}",
                plugin.plugin_id()
            )
        })?;
    let packed_output = run_json
        .call(&mut store, (request_ptr, request_json.len() as u32))
        .map_err(|error| {
            if error.to_string().contains("epoch deadline") {
                format!(
                    "code_runner node '{node_id}' exceeded timeout of {}ms",
                    node_config.timeout_ms
                )
            } else {
                format!("code_runner node '{node_id}' execution failed: {error}")
            }
        })?;
    let _ = dealloc.call(&mut store, (request_ptr, request_json.len() as u32));

    let (output_ptr, output_len) = unpack_ptr_len(packed_output);
    let response_json = read_core_memory_bytes(&memory, &mut store, output_ptr, output_len)
        .map_err(|error| {
            format!(
                "code_runner plugin '{}' failed to read output buffer: {error}",
                plugin.plugin_id()
            )
        })?;
    let _ = dealloc.call(&mut store, (output_ptr, output_len));
    let envelope =
        serde_json::from_slice::<CoreCodeRunnerResponse>(&response_json).map_err(|error| {
            format!("code_runner node '{node_id}' returned invalid runtime envelope: {error}")
        })?;

    if !envelope.ok {
        return Err(format!(
            "code_runner node '{node_id}' execution failed: {}",
            envelope
                .error
                .unwrap_or_else(|| "unknown guest error".to_string())
        ));
    }

    parse_code_runner_output(node_id, envelope.json.as_deref().unwrap_or("{}"))
}

fn build_code_runner_input(input: &RuntimeExecuteInput) -> CodeRunnerInput<'_> {
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

fn normalize_code_runner_source(source: &str) -> String {
    let trimmed = source.trim_start();
    if trimmed.starts_with("export function run") {
        source.replacen("export function run", "function run", 1)
    } else {
        source.to_string()
    }
}

fn parse_code_runner_output(node_id: &str, json: &str) -> Result<RuntimeExecuteOutput, String> {
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
                CodeRunnerHeaderOp::Append { name, value } => {
                    RuntimeHeaderOp::Append { name, value }
                }
                CodeRunnerHeaderOp::Remove { name } => RuntimeHeaderOp::Remove { name },
            })
            .collect(),
        path_rewrite: output.path_rewrite,
        next_port: output.next_port,
        logs: output
            .logs
            .into_iter()
            .map(|log| RuntimeLogEntry {
                level: match log.level {
                    CodeRunnerLogLevel::Debug => RuntimeLogLevel::Debug,
                    CodeRunnerLogLevel::Info => RuntimeLogLevel::Info,
                    CodeRunnerLogLevel::Warn => RuntimeLogLevel::Warn,
                    CodeRunnerLogLevel::Error => RuntimeLogLevel::Error,
                },
                message: log.message,
            })
            .collect(),
    })
}

fn unpack_ptr_len(packed: u64) -> (u32, u32) {
    ((packed & 0xffff_ffff) as u32, (packed >> 32) as u32)
}

fn read_core_memory_bytes(
    memory: &Memory,
    store: &mut Store<CorePluginStoreData>,
    ptr: u32,
    len: u32,
) -> Result<Vec<u8>, wasmtime::MemoryAccessError> {
    if len == 0 {
        return Ok(Vec::new());
    }
    let mut bytes = vec![0; len as usize];
    memory.read(store, ptr as usize, &mut bytes)?;
    Ok(bytes)
}

fn apply_output(
    node_id: &str,
    output: &RuntimeExecuteOutput,
    workflow_context: &mut HashMap<String, String>,
    outgoing_headers: &mut HashMap<String, Vec<String>>,
    resolved_path: &mut String,
) {
    log_runtime_output(node_id, None, output);
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
