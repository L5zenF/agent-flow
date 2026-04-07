use super::{
    plugin_workspace_root, resolve_plugin_network_policy, resolve_plugin_preopens,
    socket_addr_allowed, GatewayGraphExecutor, PluginNodeRuntime, WASMTIME_PLUGIN_RUNTIME,
};
use crate::config::{
    parse_config, GatewayConfig, LoadedWorkflowSet, ProviderConfig, WasmCapability,
    WasmPluginNodeConfig, WorkflowFileConfig, WorkflowIndexEntry,
};
use crate::gateway_execution::{execute_rule_graph, resolve_request, RequestResolution};
use axum::http::{HeaderMap, HeaderName, HeaderValue, Method, Uri};
use infrastructure::plugin_registry::{load_plugin_registry, PluginRegistry};
use infrastructure::plugin_runtime_contract::{
    RuntimeContextPatchOp, RuntimeExecuteInput, RuntimeExecuteOutput, RuntimeNodeConfig,
    RuntimePluginManifest,
};
use std::collections::BTreeMap;
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use wasmtime_wasi::sockets::SocketAddrUse;
use wasmtime_wasi::{DirPerms, FilePerms};

const TEST_COMPONENT_BYTES: &[u8] = &[
    0x00, 0x61, 0x73, 0x6d, 0x0d, 0x00, 0x01, 0x00, 0x07, 0x10, 0x01, 0x41, 0x02, 0x01, 0x42,
    0x00, 0x04, 0x01, 0x05, 0x65, 0x3a, 0x65, 0x2f, 0x65, 0x05, 0x00, 0x0b, 0x07, 0x01, 0x00,
    0x01, 0x65, 0x03, 0x00, 0x00,
];
static NEXT_TEMP_ID: AtomicUsize = AtomicUsize::new(0);

struct FakePluginRuntime {
    result: Result<RuntimeExecuteOutput, String>,
    calls: AtomicUsize,
}

impl FakePluginRuntime {
    fn succeeds(result: RuntimeExecuteOutput) -> Self {
        Self {
            result: Ok(result),
            calls: AtomicUsize::new(0),
        }
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl PluginNodeRuntime for FakePluginRuntime {
    fn execute(
        &self,
        _plugin: &infrastructure::plugin_registry::LoadedPlugin,
        _timeout_ms: u64,
        _fuel: Option<u64>,
        _max_memory_bytes: u64,
        _node_config: &WasmPluginNodeConfig,
        _input: &RuntimeExecuteInput,
    ) -> Result<RuntimeExecuteOutput, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.result.clone()
    }
}

fn temp_dir(name: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be monotonic enough for tests")
        .as_nanos();
    let counter = NEXT_TEMP_ID.fetch_add(1, Ordering::SeqCst);
    dir.push(format!(
        "proxy-tools-gateway-{name}-{}-{stamp}-{counter}",
        std::process::id(),
    ));
    fs::create_dir_all(&dir).expect("temp dir should be creatable");
    dir
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root should resolve")
}

fn write_plugin(root: &Path, id: &str, ports: &[&str], capabilities: &[&str]) {
    let plugin_dir = root.join(id);
    fs::create_dir_all(&plugin_dir).expect("plugin dir should be creatable");
    let supported_output_ports = ports
        .iter()
        .map(|port| format!("\"{port}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let capability_values = capabilities
        .iter()
        .map(|capability| format!("\"{capability}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let manifest = format!(
        r#"
id = "{id}"
name = "Intent Classifier"
version = "1.0.0"
description = "Tests gateway wasm execution"
supported_output_ports = [{supported_output_ports}]
capabilities = [{capability_values}]
"#
    );
    fs::write(plugin_dir.join("plugin.toml"), manifest).expect("manifest should write");
    let wasm_dir = plugin_dir.join("wasm");
    fs::create_dir_all(&wasm_dir).expect("wasm dir should be creatable");
    fs::write(wasm_dir.join("plugin.wasm"), TEST_COMPONENT_BYTES).expect("wasm should write");
}

fn copy_repo_plugin(root: &Path, id: &str) {
    let source_dir = workspace_root().join("plugins").join(id);
    let target_dir = root.join(id);
    let source_wasm = {
        let nested = source_dir.join("wasm").join("plugin.wasm");
        if nested.is_file() {
            nested
        } else {
            source_dir.join("plugin.wasm")
        }
    };
    let target_wasm_dir = target_dir.join("wasm");
    fs::create_dir_all(&target_dir).expect("plugin dir should be creatable");
    fs::create_dir_all(&target_wasm_dir).expect("plugin wasm dir should be creatable");
    fs::copy(source_dir.join("plugin.toml"), target_dir.join("plugin.toml"))
        .expect("plugin manifest should copy");
    fs::copy(source_wasm, target_wasm_dir.join("plugin.wasm")).expect("plugin wasm should copy");
}

fn load_test_registry(ports: &[&str], capabilities: &[&str]) -> PluginRegistry {
    let root = temp_dir("plugins");
    write_plugin(&root, "intent-classifier", ports, capabilities);
    copy_repo_plugin(&root, "js-code-runner");
    load_plugin_registry(&root).expect("plugin registry should load")
}

fn parse_graph_config(plugin_settings: &str, extra_nodes: &str, extra_edges: &str) -> GatewayConfig {
    parse_config(&format!(
        r#"
listen = "127.0.0.1:9001"
admin_listen = "127.0.0.1:9002"

[[providers]]
id = "kimi"
name = "Kimi"
base_url = "https://api.kimi.com"

[rule_graph]
version = 1
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"
position = {{ x = 0.0, y = 0.0 }}

[[rule_graph.nodes]]
id = "plugin"
type = "wasm_plugin"
position = {{ x = 120.0, y = 0.0 }}

[rule_graph.nodes.wasm_plugin]
plugin_id = "intent-classifier"
max_memory_bytes = 16777216
{plugin_settings}

{extra_nodes}

[[rule_graph.nodes]]
id = "end"
type = "end"
position = {{ x = 480.0, y = 0.0 }}

[[rule_graph.edges]]
id = "edge-start-plugin"
source = "start"
target = "plugin"

{extra_edges}
"#
    ))
    .expect("graph config should parse")
}

fn execute_test_graph<'a>(
    config: &'a GatewayConfig,
    registry: &PluginRegistry,
    runtime: &dyn PluginNodeRuntime,
    path: &str,
) -> Result<RequestResolution<'a>, String> {
    execute_test_graph_with_headers(config, registry, runtime, path, &[])
}

fn execute_test_graph_with_headers<'a>(
    config: &'a GatewayConfig,
    registry: &PluginRegistry,
    runtime: &dyn PluginNodeRuntime,
    path: &str,
    headers: &[(&str, &str)],
) -> Result<RequestResolution<'a>, String> {
    let graph = config.rule_graph.as_ref().expect("graph should exist");
    let method = Method::POST;
    let uri = path.parse::<Uri>().expect("path should parse");
    let mut request_headers = HeaderMap::new();
    for (name, value) in headers {
        request_headers.insert(
            HeaderName::from_bytes(name.as_bytes()).expect("header name should parse"),
            HeaderValue::from_str(value).expect("header value should parse"),
        );
    }
    let executor = GatewayGraphExecutor { runtime };
    execute_rule_graph(
        config,
        registry,
        &executor,
        graph,
        &method,
        &uri,
        &request_headers,
    )
}

fn header_values(resolution: &RequestResolution<'_>, name: &str) -> Vec<String> {
    resolution
        .extra_headers
        .iter()
        .filter(|(header_name, _): &&(HeaderName, HeaderValue)| {
            header_name.as_str().eq_ignore_ascii_case(name)
        })
        .map(|(_, value): &(HeaderName, HeaderValue)| {
            value
                .to_str()
                .expect("header value should be utf-8")
                .to_string()
        })
        .collect()
}

include!("tests_rest.inc");
