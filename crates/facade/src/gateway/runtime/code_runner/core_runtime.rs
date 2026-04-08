use ::wasmtime::Linker as CoreLinker;
use ::wasmtime::{Memory, Store, StoreLimitsBuilder};
use infrastructure::plugin_registry::LoadedPlugin;
use infrastructure::plugin_runtime_contract::{
    CoreCodeRunnerRequest, RuntimeExecuteInput, RuntimeExecuteOutput,
};
use serde::Deserialize;
use wasmtime_wasi::WasiCtxBuilder;

use super::super::CorePluginStoreData;
use super::super::TimeoutEpochGuard;
use super::io::{build_code_runner_input, normalize_code_runner_source, parse_code_runner_output};
use crate::config::CodeRunnerNodeConfig;

#[derive(Debug, Deserialize)]
struct CoreCodeRunnerResponse {
    ok: bool,
    #[serde(default)]
    json: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

pub(super) fn execute_core_code_runner_plugin(
    plugin: &LoadedPlugin,
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
