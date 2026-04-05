use std::collections::{BTreeMap, HashMap, HashSet};
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{SyncSender, sync_channel};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use axum::body::{Body, to_bytes};
use axum::extract::{Request, State};
use axum::http::header::{CONNECTION, HOST};
use axum::http::{HeaderMap, HeaderName, HeaderValue, Method, Response, StatusCode, Uri};
use axum::response::IntoResponse;
use reqwest::Client;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use url::Url;
use wasmtime::component::{Linker, ResourceTable};
use wasmtime::{Store, StoreLimits, StoreLimitsBuilder};
use wasmtime_wasi::{
    DirPerms, FilePerms, SocketAddrUse, WasiCtx, WasiCtxBuilder, WasiView, add_to_linker_sync,
};

use self::exports::proxy_tools::proxy_node_plugin::node_plugin::{
    CapabilityDeclaration, CapabilityGrant, CapabilityKind, ContextEntry, ContextPatchOp,
    ExecuteError, ExecuteInput, ExecuteOutput, HeaderOp, JsonDocument, LogLevel, NodeConfig,
    PluginManifest, RequestHeader,
};
use crate::config::{
    ConditionMode, GatewayConfig, HeaderValueConfig, LoadedWorkflowSet, ModelConfig,
    ProviderConfig, RouteConfig, RouterClauseConfig, RuleGraphConfig, RuleGraphNodeType,
    WasmCapability, WasmPluginNodeConfig,
};
use crate::crypto::decrypt_header_value;
use crate::rules::{RequestContext, build_header_map, evaluate_expression, render_template};
use crate::wasm_plugins::{LoadedPlugin, ManifestCapability, PluginRegistry};

wasmtime::component::bindgen!({
    inline: r#"
        package proxy-tools:proxy-node-plugin@0.1.0;

        interface node-plugin {
          record request-header {
            name: string,
            value: string,
          }

          record context-entry {
            key: string,
            value: string,
          }

          record json-document {
            json: string,
          }

          variant context-patch-op {
            set(context-entry),
            remove(string),
          }

          record context-patch {
            ops: list<context-patch-op>,
          }

          variant header-op {
            set(request-header),
            append(request-header),
            remove(string),
          }

          record path-rewrite {
            path: string,
          }

          record next-port {
            port: string,
          }

          variant log-level {
            debug,
            info,
            warn,
            error,
          }

          record log-entry {
            level: log-level,
            message: string,
          }

          variant capability-kind {
            log,
            fs,
            network,
          }

          record capability-declaration {
            kind: capability-kind,
            required: bool,
            scope: option<string>,
            description: string,
          }

          record capability-grant {
            kind: capability-kind,
            allowed: bool,
            scope: option<string>,
          }

          record plugin-manifest {
            id: string,
            name: string,
            version: string,
            description: string,
            supported-output-ports: list<string>,
            default-config-schema-hints: option<json-document>,
            capabilities: list<capability-declaration>,
          }

          record node-config {
            manifest: plugin-manifest,
            grants: list<capability-grant>,
            config: option<json-document>,
          }

          record execute-input {
            request-method: string,
            current-path: string,
            request-headers: list<request-header>,
            workflow-context: list<context-entry>,
            selected-provider-id: string,
            selected-model-id: string,
            node-config: node-config,
          }

          record execute-output {
            context-patch: option<context-patch>,
            header-ops: list<header-op>,
            path-rewrite: option<path-rewrite>,
            next-port: option<next-port>,
            logs: list<log-entry>,
          }

          variant execute-error {
            denied(string),
            invalid-input(string),
            failed(string),
          }

          execute: func(input: execute-input) -> result<execute-output, execute-error>;
        }

        world proxy-node-plugin {
          export node-plugin;
        }
    "#,
    world: "proxy-node-plugin",
    ownership: Owning,
});

#[derive(Clone)]
pub struct GatewayState {
    pub client: Client,
    pub config: Arc<RwLock<GatewayConfig>>,
    pub workflow_store: Arc<RwLock<LoadedWorkflowSet>>,
    pub plugin_registry: Arc<PluginRegistry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeRequestHeader {
    name: String,
    value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum RuntimeCapabilityKind {
    Log,
    Fs,
    Network,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeCapabilityDeclaration {
    kind: RuntimeCapabilityKind,
    required: bool,
    scope: Option<String>,
    description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeCapabilityGrant {
    kind: RuntimeCapabilityKind,
    allowed: bool,
    scope: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimePluginManifest {
    id: String,
    name: String,
    version: String,
    description: String,
    supported_output_ports: Vec<String>,
    default_config_schema_hints_json: Option<String>,
    capabilities: Vec<RuntimeCapabilityDeclaration>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeNodeConfig {
    manifest: RuntimePluginManifest,
    grants: Vec<RuntimeCapabilityGrant>,
    config_json: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeExecuteInput {
    request_method: String,
    current_path: String,
    request_headers: Vec<RuntimeRequestHeader>,
    workflow_context: Vec<(String, String)>,
    selected_provider_id: String,
    selected_model_id: String,
    node_config: RuntimeNodeConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RuntimeContextPatchOp {
    Set { key: String, value: String },
    Remove { key: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RuntimeHeaderOp {
    Set { name: String, value: String },
    Append { name: String, value: String },
    Remove { name: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RuntimeLogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeLogEntry {
    level: RuntimeLogLevel,
    message: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct RuntimeExecuteOutput {
    context_ops: Vec<RuntimeContextPatchOp>,
    header_ops: Vec<RuntimeHeaderOp>,
    path_rewrite: Option<String>,
    next_port: Option<String>,
    logs: Vec<RuntimeLogEntry>,
}

trait PluginNodeRuntime {
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

struct WasmtimePluginRuntime;

struct PluginStoreData {
    limits: StoreLimits,
    table: ResourceTable,
    wasi: WasiCtx,
}

static WASMTIME_PLUGIN_RUNTIME: WasmtimePluginRuntime = WasmtimePluginRuntime;

impl WasiView for PluginStoreData {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }

    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi
    }
}

#[derive(Debug, Clone)]
struct PluginPreopenDir {
    host_path: PathBuf,
    guest_path: String,
    dir_perms: DirPerms,
    file_perms: FilePerms,
}

#[derive(Clone)]
struct PluginNetworkPolicy {
    allowed_addrs: Arc<HashSet<SocketAddr>>,
    allow_ip_name_lookup: bool,
}

struct TimeoutEpochGuard {
    cancel: Option<SyncSender<()>>,
    handle: Option<JoinHandle<()>>,
}

impl TimeoutEpochGuard {
    fn start(engine: wasmtime::Engine, timeout: Duration) -> Self {
        let (cancel, receiver) = sync_channel::<()>(1);
        let handle = thread::spawn(move || {
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
        let mut linker = Linker::new(plugin.component().engine());
        add_to_linker_sync(&mut linker).map_err(|error| {
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
                table: ResourceTable::new(),
                wasi,
            },
        );
        store.limiter(|state| &mut state.limits);
        if let Some(fuel) = fuel {
            store
                .set_fuel(fuel)
                .map_err(|error| format!("failed to set plugin fuel budget: {error}"))?;
        }
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
            Duration::from_millis(timeout_ms),
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

pub async fn proxy_request(
    State(state): State<GatewayState>,
    request: Request,
) -> impl IntoResponse {
    let method = request.method().clone();
    let headers = request.headers().clone();
    let uri = request.uri().clone();
    let body = match to_bytes(request.into_body(), usize::MAX).await {
        Ok(body) => body,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                format!("failed to read request body: {error}"),
            )
                .into_response();
        }
    };

    let config = state.config.read().await;
    let workflow_store = state.workflow_store.read().await;
    let resolution = match resolve_request(
        &config,
        &workflow_store,
        state.plugin_registry.as_ref(),
        &WASMTIME_PLUGIN_RUNTIME,
        &method,
        &uri,
        &headers,
    ) {
        Ok(Some(value)) => value,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                "no route matched request".to_string(),
            )
                .into_response();
        }
        Err(error) => return (StatusCode::BAD_REQUEST, error).into_response(),
    };

    let upstream_url =
        match build_upstream_url(&resolution.provider.base_url, &resolution.path, uri.query()) {
            Ok(url) => url,
            Err(error) => {
                return (
                    StatusCode::BAD_GATEWAY,
                    format!(
                        "invalid upstream url for provider '{}': {error}",
                        resolution.provider.id
                    ),
                )
                    .into_response();
            }
        };

    info!(
        route_id = %resolution.route.map(|item| item.id.as_str()).unwrap_or("<rule-graph>"),
        provider_id = %resolution.provider.id,
        model_id = %resolution.model.map(|item| item.id.as_str()).unwrap_or("<none>"),
        method = %method,
        upstream = %upstream_url,
        injected_headers = %format_header_names(&resolution.extra_headers),
        "forwarding request"
    );

    match forward_request(
        &state.client,
        method,
        headers.clone(),
        uri,
        upstream_url,
        body,
        &resolution.extra_headers,
    )
    .await
    {
        Ok(response) => response.into_response(),
        Err((status, message)) => (status, message).into_response(),
    }
}

#[derive(Debug)]
struct RequestResolution<'a> {
    provider: &'a ProviderConfig,
    model: Option<&'a ModelConfig>,
    path: String,
    extra_headers: Vec<(HeaderName, HeaderValue)>,
    route: Option<&'a RouteConfig>,
}

fn resolve_request<'a>(
    config: &'a GatewayConfig,
    workflow_store: &'a LoadedWorkflowSet,
    plugin_registry: &PluginRegistry,
    runtime: &dyn PluginNodeRuntime,
    method: &Method,
    uri: &Uri,
    headers: &'a HeaderMap,
) -> Result<Option<RequestResolution<'a>>, String> {
    let mut workflow_context = HashMap::new();
    inject_runtime_context(
        &mut workflow_context,
        method.as_str(),
        uri.path(),
        headers,
        None,
        None,
        None,
    );
    if let Some(graph) = workflow_store.active_graph() {
        if !graph.nodes.is_empty() {
            return execute_rule_graph(
                config,
                plugin_registry,
                runtime,
                graph,
                method,
                uri,
                headers,
            )
            .map(Some);
        }
    }

    let Some((route, provider, model)) = resolve_route(config, method, uri, headers) else {
        return Ok(None);
    };

    let request_context = RequestContext {
        method: method.as_str(),
        path: uri.path(),
        headers,
        context: &workflow_context,
        provider: Some(provider),
        model,
        route: Some(route),
    };

    Ok(Some(RequestResolution {
        provider,
        model,
        path: route
            .path_rewrite
            .as_deref()
            .unwrap_or(uri.path())
            .to_string(),
        extra_headers: build_header_map(config, &request_context)?,
        route: Some(route),
    }))
}

fn execute_rule_graph<'a>(
    config: &'a GatewayConfig,
    plugin_registry: &PluginRegistry,
    runtime: &dyn PluginNodeRuntime,
    graph: &'a RuleGraphConfig,
    method: &Method,
    uri: &Uri,
    headers: &HeaderMap,
) -> Result<RequestResolution<'a>, String> {
    let node_map = graph
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect::<HashMap<_, _>>();
    let fallback_route = config.routes.first();
    let mut current_id = graph.start_node_id.as_str();
    let mut selected_provider: Option<&ProviderConfig> = None;
    let mut selected_model: Option<&ModelConfig> = None;
    let mut resolved_path = uri.path().to_string();
    let mut workflow_context = HashMap::<String, String>::new();
    let mut outgoing_headers = HashMap::<String, Vec<String>>::new();
    let mut traversed = HashSet::<String>::new();
    let max_steps = graph.nodes.len().saturating_mul(3).max(1);

    for _ in 0..max_steps {
        let node = node_map
            .get(current_id)
            .copied()
            .ok_or_else(|| format!("rule_graph node '{}' does not exist", current_id))?;
        traversed.insert(current_id.to_string());
        inject_runtime_context(
            &mut workflow_context,
            method.as_str(),
            &resolved_path,
            headers,
            selected_provider,
            selected_model,
            fallback_route,
        );

        let request_context = RequestContext {
            method: method.as_str(),
            path: &resolved_path,
            headers,
            context: &workflow_context,
            provider: selected_provider,
            model: selected_model,
            route: fallback_route,
        };

        let next = match node.node_type {
            RuleGraphNodeType::Start => next_linear_edge(graph, current_id)?,
            RuleGraphNodeType::Note => next_linear_edge(graph, current_id)?,
            RuleGraphNodeType::Condition => {
                let condition = node.condition.as_ref().ok_or_else(|| {
                    format!("rule_graph node '{}' missing condition config", node.id)
                })?;
                let expression = match condition.mode {
                    ConditionMode::Expression => condition.expression.clone().ok_or_else(|| {
                        format!("rule_graph node '{}' missing expression", node.id)
                    })?,
                    ConditionMode::Builder => {
                        let builder = condition.builder.as_ref().ok_or_else(|| {
                            format!("rule_graph node '{}' missing builder config", node.id)
                        })?;
                        format!(
                            "{} {} \"{}\"",
                            builder.field, builder.operator, builder.value
                        )
                    }
                };
                let branch = if evaluate_expression(&expression, &request_context)? {
                    "true"
                } else {
                    "false"
                };
                next_condition_edge(graph, current_id, branch)?
            }
            RuleGraphNodeType::RouteProvider => {
                let provider_id = node
                    .route_provider
                    .as_ref()
                    .ok_or_else(|| {
                        format!(
                            "rule_graph node '{}' missing route_provider config",
                            node.id
                        )
                    })?
                    .provider_id
                    .as_str();
                selected_provider = Some(
                    config
                        .providers
                        .iter()
                        .find(|provider| provider.id == provider_id)
                        .ok_or_else(|| {
                            format!("rule_graph provider '{}' not found", provider_id)
                        })?,
                );

                if let Some(provider) = selected_provider {
                    for header in &provider.default_headers {
                        let value = resolve_provider_header_for_graph(header, config)?;
                        outgoing_headers.insert(header.name.to_ascii_lowercase(), vec![value]);
                    }
                }
                next_linear_edge(graph, current_id)?
            }
            RuleGraphNodeType::SelectModel => {
                let select_model = node.select_model.as_ref().ok_or_else(|| {
                    format!("rule_graph node '{}' missing select_model config", node.id)
                })?;
                let provider_id = select_model.provider_id.as_str();
                let model_id = select_model.model_id.as_str();
                selected_provider = Some(
                    config
                        .providers
                        .iter()
                        .find(|provider| provider.id == provider_id)
                        .ok_or_else(|| {
                            format!("rule_graph provider '{}' not found", provider_id)
                        })?,
                );
                selected_model = Some(
                    config
                        .models
                        .iter()
                        .find(|model| model.id == model_id)
                        .ok_or_else(|| format!("rule_graph model '{}' not found", model_id))?,
                );
                if let (Some(provider), Some(model)) = (selected_provider, selected_model) {
                    if model.provider_id != provider.id {
                        return Err(format!(
                            "rule_graph model '{}' does not belong to provider '{}'",
                            model.id, provider.id
                        ));
                    }
                    for header in &provider.default_headers {
                        let value = resolve_provider_header_for_graph(header, config)?;
                        outgoing_headers.insert(header.name.to_ascii_lowercase(), vec![value]);
                    }
                }
                next_linear_edge(graph, current_id)?
            }
            RuleGraphNodeType::RewritePath => {
                let node_config = node.rewrite_path.as_ref().ok_or_else(|| {
                    format!("rule_graph node '{}' missing rewrite_path config", node.id)
                })?;
                resolved_path = render_template(&node_config.value, &request_context)?;
                next_linear_edge(graph, current_id)?
            }
            RuleGraphNodeType::SetContext => {
                let node_config = node.set_context.as_ref().ok_or_else(|| {
                    format!("rule_graph node '{}' missing set_context config", node.id)
                })?;
                let value = render_template(&node_config.value_template, &request_context)?;
                workflow_context.insert(node_config.key.clone(), value);
                next_linear_edge(graph, current_id)?
            }
            RuleGraphNodeType::WasmPlugin => {
                let node_config = node.wasm_plugin.as_ref().ok_or_else(|| {
                    format!("rule_graph node '{}' missing wasm_plugin config", node.id)
                })?;
                let plugin = plugin_registry
                    .get(node_config.plugin_id.as_str())
                    .ok_or_else(|| {
                        format!(
                            "rule_graph node '{}' references unknown plugin '{}'",
                            node.id, node_config.plugin_id
                        )
                    })?;
                let runtime_input = build_runtime_execute_input(
                    plugin,
                    node.id.as_str(),
                    node_config,
                    method.as_str(),
                    &resolved_path,
                    &current_request_headers(headers, &outgoing_headers),
                    &workflow_context,
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
                    &node.id,
                    plugin,
                    &runtime_output,
                    &mut workflow_context,
                    &mut outgoing_headers,
                    &mut resolved_path,
                );

                match runtime_output.next_port.as_deref() {
                    Some(port) => {
                        if !plugin
                            .manifest()
                            .supported_output_ports
                            .iter()
                            .any(|item| item == port)
                        {
                            return Err(format!(
                                "plugin '{}' returned unknown port '{}' for node '{}'",
                                plugin.plugin_id(),
                                port,
                                node.id
                            ));
                        }
                        Some(next_condition_edge(graph, current_id, port)?.ok_or_else(|| {
                            format!(
                                "plugin '{}' returned port '{}' for node '{}' but no outgoing edge matched it",
                                plugin.plugin_id(),
                                port,
                                node.id
                            )
                        })?)
                    }
                    None => next_condition_edge(graph, current_id, "default")?
                        .or(next_linear_edge(graph, current_id)?),
                }
            }
            RuleGraphNodeType::Router => {
                let node_config = node.router.as_ref().ok_or_else(|| {
                    format!("rule_graph node '{}' missing router config", node.id)
                })?;
                let mut matched_target = None;
                for rule in &node_config.rules {
                    if rule.clauses.iter().all(|clause| {
                        evaluate_router_clause(clause, &request_context).unwrap_or(false)
                    }) {
                        matched_target = Some(rule.target_node_id.as_str());
                        break;
                    }
                }
                Some(
                    matched_target
                        .or(node_config.fallback_node_id.as_deref())
                        .ok_or_else(|| {
                            format!(
                                "rule_graph router '{}' matched no rule and has no fallback target",
                                node.id
                            )
                        })?,
                )
            }
            RuleGraphNodeType::Log => {
                let node_config = node
                    .log
                    .as_ref()
                    .ok_or_else(|| format!("rule_graph node '{}' missing log config", node.id))?;
                let message = render_template(&node_config.message, &request_context)?;
                info!(node_id = %node.id, message = %message, "rule graph log");
                next_linear_edge(graph, current_id)?
            }
            RuleGraphNodeType::SetHeader => {
                let node_config = node.set_header.as_ref().ok_or_else(|| {
                    format!("rule_graph node '{}' missing set_header config", node.id)
                })?;
                outgoing_headers.insert(
                    node_config.name.to_ascii_lowercase(),
                    vec![render_template(&node_config.value, &request_context)?],
                );
                next_linear_edge(graph, current_id)?
            }
            RuleGraphNodeType::RemoveHeader => {
                let node_config = node.remove_header.as_ref().ok_or_else(|| {
                    format!("rule_graph node '{}' missing remove_header config", node.id)
                })?;
                outgoing_headers.remove(&node_config.name.to_ascii_lowercase());
                next_linear_edge(graph, current_id)?
            }
            RuleGraphNodeType::CopyHeader => {
                let node_config = node.copy_header.as_ref().ok_or_else(|| {
                    format!("rule_graph node '{}' missing copy_header config", node.id)
                })?;
                let source = headers
                    .get(node_config.from.as_str())
                    .and_then(|value| value.to_str().ok())
                    .ok_or_else(|| {
                        format!(
                            "header '{}' is unavailable for graph copy action",
                            node_config.from
                        )
                    })?;
                outgoing_headers.insert(
                    node_config.to.to_ascii_lowercase(),
                    vec![source.to_string()],
                );
                next_linear_edge(graph, current_id)?
            }
            RuleGraphNodeType::SetHeaderIfAbsent => {
                let node_config = node.set_header_if_absent.as_ref().ok_or_else(|| {
                    format!(
                        "rule_graph node '{}' missing set_header_if_absent config",
                        node.id
                    )
                })?;
                if !outgoing_headers.contains_key(&node_config.name.to_ascii_lowercase()) {
                    outgoing_headers.insert(
                        node_config.name.to_ascii_lowercase(),
                        vec![render_template(&node_config.value, &request_context)?],
                    );
                }
                next_linear_edge(graph, current_id)?
            }
            RuleGraphNodeType::End => break,
        };

        current_id = match next {
            Some(next_id) => next_id,
            None => break,
        };
    }

    if traversed.len() >= max_steps {
        return Err("rule_graph exceeded maximum execution steps".to_string());
    }

    let provider =
        selected_provider.ok_or_else(|| "rule_graph did not select a provider".to_string())?;
    let mut extra_headers = Vec::new();
    for (name, values) in outgoing_headers {
        let header_name = HeaderName::try_from(name).map_err(|error| error.to_string())?;
        for value in values {
            let header_value = HeaderValue::from_str(&value).map_err(|error| error.to_string())?;
            extra_headers.push((header_name.clone(), header_value));
        }
    }

    Ok(RequestResolution {
        provider,
        model: selected_model,
        path: resolved_path,
        extra_headers,
        route: None,
    })
}

fn resolve_provider_header_for_graph(
    header: &crate::config::HeaderConfig,
    config: &GatewayConfig,
) -> Result<String, String> {
    match &header.value {
        HeaderValueConfig::Plain { value } => Ok(value.clone()),
        HeaderValueConfig::Encrypted {
            value,
            encrypted: true,
            secret_env,
        } => decrypt_header_value(
            value,
            secret_env
                .as_deref()
                .or(config.default_secret_env.as_deref())
                .ok_or_else(|| {
                    format!(
                        "header '{}' is encrypted but missing secret_env",
                        header.name
                    )
                })?,
        ),
        HeaderValueConfig::Encrypted { value, .. } => Ok(value.clone()),
    }
}

fn next_linear_edge<'a>(
    graph: &'a RuleGraphConfig,
    node_id: &str,
) -> Result<Option<&'a str>, String> {
    let edges = graph
        .edges
        .iter()
        .filter(|edge| edge.source == node_id)
        .collect::<Vec<_>>();
    if edges.len() > 1 {
        return Err(format!(
            "rule_graph node '{}' has multiple outgoing edges but is not a condition node",
            node_id
        ));
    }
    Ok(edges.first().map(|edge| edge.target.as_str()))
}

fn next_condition_edge<'a>(
    graph: &'a RuleGraphConfig,
    node_id: &str,
    branch: &str,
) -> Result<Option<&'a str>, String> {
    Ok(graph
        .edges
        .iter()
        .find(|edge| edge.source == node_id && edge.source_handle.as_deref() == Some(branch))
        .map(|edge| edge.target.as_str()))
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
    for log in &output.logs {
        match log.level {
            RuntimeLogLevel::Debug => info!(
                node_id = %node_id,
                plugin_id = %plugin.plugin_id(),
                message = %log.message,
                "wasm plugin debug"
            ),
            RuntimeLogLevel::Info => info!(
                node_id = %node_id,
                plugin_id = %plugin.plugin_id(),
                message = %log.message,
                "wasm plugin log"
            ),
            RuntimeLogLevel::Warn => warn!(
                node_id = %node_id,
                plugin_id = %plugin.plugin_id(),
                message = %log.message,
                "wasm plugin warning"
            ),
            RuntimeLogLevel::Error => error!(
                node_id = %node_id,
                plugin_id = %plugin.plugin_id(),
                message = %log.message,
                "wasm plugin error"
            ),
        }
    }

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

fn build_plugin_wasi_ctx(
    plugin: &LoadedPlugin,
    node_config: &WasmPluginNodeConfig,
) -> Result<WasiCtx, String> {
    let mut builder = WasiCtxBuilder::new();
    builder.allow_blocking_current_thread(true);
    builder.allow_tcp(false);
    builder.allow_udp(false);
    builder.allow_ip_name_lookup(false);

    for preopen in resolve_plugin_preopens(plugin, node_config)? {
        builder
            .preopened_dir(
                &preopen.host_path,
                preopen.guest_path.as_str(),
                preopen.dir_perms,
                preopen.file_perms,
            )
            .map_err(|error| {
                format!(
                    "failed to preopen '{}' for plugin '{}': {error}",
                    preopen.host_path.display(),
                    plugin.plugin_id()
                )
            })?;
    }

    if let Some(policy) = resolve_plugin_network_policy(node_config)? {
        let allowed_addrs = policy.allowed_addrs.clone();
        builder.allow_tcp(true);
        builder.allow_udp(true);
        builder.allow_ip_name_lookup(policy.allow_ip_name_lookup);
        builder.socket_addr_check(move |addr, reason| {
            let allowed_addrs = allowed_addrs.clone();
            Box::pin(async move { socket_addr_allowed(&allowed_addrs, addr, reason) })
        });
    }

    Ok(builder.build())
}

fn resolve_plugin_preopens(
    plugin: &LoadedPlugin,
    node_config: &WasmPluginNodeConfig,
) -> Result<Vec<PluginPreopenDir>, String> {
    let root = plugin_workspace_root(plugin);
    let mut preopens = BTreeMap::<String, PluginPreopenDir>::new();

    for relative in &node_config.read_dirs {
        upsert_preopen_dir(
            &mut preopens,
            &root,
            relative,
            DirPerms::READ,
            FilePerms::READ,
        );
    }

    for relative in &node_config.write_dirs {
        upsert_preopen_dir(
            &mut preopens,
            &root,
            relative,
            DirPerms::READ | DirPerms::MUTATE,
            FilePerms::READ | FilePerms::WRITE,
        );
    }

    Ok(preopens.into_values().collect())
}

fn upsert_preopen_dir(
    preopens: &mut BTreeMap<String, PluginPreopenDir>,
    root: &Path,
    relative: &str,
    dir_perms: DirPerms,
    file_perms: FilePerms,
) {
    let guest_path = guest_preopen_path(relative);
    let host_path = root.join(relative);

    preopens
        .entry(guest_path.clone())
        .and_modify(|existing| {
            existing.dir_perms |= dir_perms;
            existing.file_perms |= file_perms;
        })
        .or_insert_with(|| PluginPreopenDir {
            host_path,
            guest_path,
            dir_perms,
            file_perms,
        });
}

fn guest_preopen_path(relative: &str) -> String {
    let trimmed = relative.trim_matches('/');
    if trimmed.is_empty() {
        "/".to_string()
    } else {
        format!("/{trimmed}")
    }
}

fn plugin_workspace_root(plugin: &LoadedPlugin) -> PathBuf {
    plugin
        .directory()
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| plugin.directory().to_path_buf())
}

fn resolve_plugin_network_policy(
    node_config: &WasmPluginNodeConfig,
) -> Result<Option<PluginNetworkPolicy>, String> {
    if !node_config
        .granted_capabilities
        .iter()
        .any(|capability| matches!(capability, WasmCapability::Network))
    {
        return Ok(None);
    }

    let mut allowed_addrs = HashSet::new();
    let mut allow_ip_name_lookup = false;

    for host in &node_config.allowed_hosts {
        if host.parse::<SocketAddr>().is_err() {
            allow_ip_name_lookup = true;
        }

        let resolved = host
            .to_socket_addrs()
            .map_err(|error| format!("failed to resolve allowlisted host '{host}': {error}"))?;
        let mut resolved_any = false;
        for addr in resolved {
            allowed_addrs.insert(addr);
            resolved_any = true;
        }

        if !resolved_any {
            return Err(format!(
                "allowlisted host '{host}' resolved to no socket addresses"
            ));
        }
    }

    Ok(Some(PluginNetworkPolicy {
        allowed_addrs: Arc::new(allowed_addrs),
        allow_ip_name_lookup,
    }))
}

fn socket_addr_allowed(
    allowed_addrs: &HashSet<SocketAddr>,
    addr: SocketAddr,
    reason: SocketAddrUse,
) -> bool {
    match reason {
        SocketAddrUse::TcpBind | SocketAddrUse::UdpBind => false,
        SocketAddrUse::TcpConnect
        | SocketAddrUse::UdpConnect
        | SocketAddrUse::UdpOutgoingDatagram => allowed_addrs.contains(&addr),
    }
}

fn current_request_headers(
    incoming_headers: &HeaderMap,
    outgoing_headers: &HashMap<String, Vec<String>>,
) -> Vec<RuntimeRequestHeader> {
    let mut merged = incoming_headers
        .iter()
        .filter_map(|(name, value)| {
            value.to_str().ok().map(|value| RuntimeRequestHeader {
                name: name.as_str().to_string(),
                value: value.to_string(),
            })
        })
        .collect::<Vec<_>>();

    for (name, values) in outgoing_headers {
        merged.retain(|header| !header.name.eq_ignore_ascii_case(name));
        for value in values {
            merged.push(RuntimeRequestHeader {
                name: name.clone(),
                value: value.clone(),
            });
        }
    }

    merged
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
        WasmCapability::Fs => {
            let reads = node_config.read_dirs.join(",");
            let writes = node_config.write_dirs.join(",");
            Some(format!("read={reads};write={writes}"))
        }
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

fn to_wit_execute_input(input: &RuntimeExecuteInput) -> ExecuteInput {
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

fn from_wit_execute_output(output: ExecuteOutput) -> RuntimeExecuteOutput {
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

fn describe_execute_error(error: &ExecuteError) -> &str {
    match error {
        ExecuteError::Denied(message)
        | ExecuteError::InvalidInput(message)
        | ExecuteError::Failed(message) => message.as_str(),
    }
}

fn resolve_route<'a>(
    config: &'a GatewayConfig,
    method: &Method,
    uri: &Uri,
    headers: &'a HeaderMap,
) -> Option<(&'a RouteConfig, &'a ProviderConfig, Option<&'a ModelConfig>)> {
    let mut workflow_context = HashMap::new();
    inject_runtime_context(
        &mut workflow_context,
        method.as_str(),
        uri.path(),
        headers,
        None,
        None,
        None,
    );
    let mut routes = config
        .routes
        .iter()
        .filter(|route| route.enabled)
        .collect::<Vec<_>>();
    routes.sort_by(|left, right| right.priority.cmp(&left.priority));

    for route in routes {
        let provider = config
            .providers
            .iter()
            .find(|provider| provider.id == route.provider_id)?;
        let model = route
            .model_id
            .as_deref()
            .and_then(|model_id| config.models.iter().find(|model| model.id == model_id));
        let request_context = RequestContext {
            method: method.as_str(),
            path: uri.path(),
            headers,
            context: &workflow_context,
            provider: Some(provider),
            model,
            route: Some(route),
        };
        if evaluate_expression(&route.matcher, &request_context).ok()? {
            return Some((route, provider, model));
        }
    }

    None
}

fn inject_runtime_context(
    context: &mut HashMap<String, String>,
    method: &str,
    path: &str,
    headers: &HeaderMap,
    provider: Option<&ProviderConfig>,
    model: Option<&ModelConfig>,
    route: Option<&RouteConfig>,
) {
    context.insert("method".to_string(), method.to_string());
    context.insert("path".to_string(), path.to_string());

    context.retain(|key, _| {
        !key.starts_with("header.")
            && !matches!(
                key.as_str(),
                "provider.id" | "provider.name" | "model.id" | "route.id"
            )
    });

    for (name, value) in headers {
        if let Ok(value) = value.to_str() {
            context.insert(
                format!("header.{}", name.as_str().to_ascii_lowercase()),
                value.to_string(),
            );
        }
    }

    if let Some(provider) = provider {
        context.insert("provider.id".to_string(), provider.id.clone());
        context.insert("provider.name".to_string(), provider.name.clone());
    }
    if let Some(model) = model {
        context.insert("model.id".to_string(), model.id.clone());
    }
    if let Some(route) = route {
        context.insert("route.id".to_string(), route.id.clone());
    }
}

fn evaluate_router_clause(
    clause: &RouterClauseConfig,
    request: &RequestContext<'_>,
) -> Result<bool, String> {
    let source = clause.source.trim();
    let operator = clause.operator.trim();
    let value = clause.value.trim().replace('"', "\\\"");
    let expression = match operator {
        "==" | "!=" => format!(r#"{source} {operator} "{value}""#),
        "startsWith" | "contains" => format!(r#"{source}.{operator}("{value}")"#),
        _ => return Err(format!("unsupported router operator '{}'", clause.operator)),
    };
    evaluate_expression(&expression, request)
}

async fn forward_request(
    client: &Client,
    method: Method,
    incoming_headers: HeaderMap,
    incoming_uri: Uri,
    upstream_url: Url,
    body: bytes::Bytes,
    extra_headers: &[(HeaderName, HeaderValue)],
) -> Result<Response<Body>, (StatusCode, String)> {
    let reqwest_method =
        reqwest::Method::from_bytes(method.as_str().as_bytes()).map_err(internal_error)?;
    let mut builder = client.request(reqwest_method, upstream_url);

    for (name, value) in incoming_headers.iter() {
        if should_skip_forward_header(name, extra_headers) {
            continue;
        }
        builder = builder.header(name, value);
    }

    for (name, value) in extra_headers {
        builder = builder.header(name, value);
    }

    let upstream_response = builder.body(body).send().await.map_err(bad_gateway_error)?;
    let status = upstream_response.status();
    let response_headers = upstream_response.headers().clone();
    let response_body = Body::from_stream(upstream_response.bytes_stream());

    let mut response = Response::builder().status(status);
    for (name, value) in response_headers.iter() {
        if name == CONNECTION {
            continue;
        }
        response = response.header(name, value);
    }

    response.body(response_body).map_err(|error| {
        error!(
            method = %method,
            uri = %incoming_uri,
            "failed to build response: {error}"
        );
        internal_error(error)
    })
}

fn build_upstream_url(
    base_url: &str,
    path: &str,
    query: Option<&str>,
) -> Result<Url, url::ParseError> {
    let mut url = Url::parse(base_url)?;
    url.set_path(path);
    url.set_query(query);
    Ok(url)
}

fn should_skip_forward_header(
    name: &HeaderName,
    extra_headers: &[(HeaderName, HeaderValue)],
) -> bool {
    name == HOST
        || name == CONNECTION
        || name.as_str().eq_ignore_ascii_case("x-target")
        || extra_headers
            .iter()
            .any(|(extra_name, _)| extra_name == name)
}

fn format_header_names(headers: &[(HeaderName, HeaderValue)]) -> String {
    if headers.is_empty() {
        return "<none>".to_string();
    }
    headers
        .iter()
        .map(|(name, _)| name.as_str().to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

fn internal_error(error: impl std::fmt::Display) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}

fn bad_gateway_error(error: impl std::fmt::Display) -> (StatusCode, String) {
    (StatusCode::BAD_GATEWAY, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        PluginNodeRuntime, RequestResolution, RuntimeContextPatchOp, RuntimeExecuteInput,
        RuntimeExecuteOutput, execute_rule_graph, plugin_workspace_root,
        resolve_plugin_network_policy, resolve_plugin_preopens, socket_addr_allowed,
    };
    use crate::config::{GatewayConfig, WasmCapability, WasmPluginNodeConfig, parse_config};
    use crate::wasm_plugins::{PluginRegistry, load_plugin_registry};
    use axum::http::{HeaderMap, Method, Uri};
    use std::fs;
    use std::net::SocketAddr;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    use wasmtime_wasi::{DirPerms, FilePerms, SocketAddrUse};

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
            _plugin: &crate::wasm_plugins::LoadedPlugin,
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
        fs::write(plugin_dir.join("plugin.wasm"), TEST_COMPONENT_BYTES).expect("wasm should write");
    }

    fn load_test_registry(ports: &[&str], capabilities: &[&str]) -> PluginRegistry {
        let root = temp_dir("plugins");
        write_plugin(&root, "intent-classifier", ports, capabilities);
        load_plugin_registry(&root).expect("plugin registry should load")
    }

    fn parse_graph_config(
        plugin_settings: &str,
        extra_nodes: &str,
        extra_edges: &str,
    ) -> GatewayConfig {
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
        let graph = config.rule_graph.as_ref().expect("graph should exist");
        let method = Method::POST;
        let uri = path.parse::<Uri>().expect("path should parse");
        let headers = HeaderMap::new();
        execute_rule_graph(config, registry, runtime, graph, &method, &uri, &headers)
    }

    fn header_values(resolution: &RequestResolution<'_>, name: &str) -> Vec<String> {
        resolution
            .extra_headers
            .iter()
            .filter(|(header_name, _)| header_name.as_str().eq_ignore_ascii_case(name))
            .map(|(_, value)| {
                value
                    .to_str()
                    .expect("header value should be utf-8")
                    .to_string()
            })
            .collect()
    }

    #[test]
    fn executes_wasm_plugin_node_that_sets_context_and_chooses_next_port() {
        let config = parse_graph_config(
            "",
            r#"
[[rule_graph.nodes]]
id = "code-header"
type = "set_header"
position = { x = 240.0, y = 0.0 }

[rule_graph.nodes.set_header]
name = "X-Intent"
value = "${ctx.intent}"

[[rule_graph.nodes]]
id = "fallback-header"
type = "set_header"
position = { x = 240.0, y = 120.0 }

[rule_graph.nodes.set_header]
name = "X-Intent"
value = "fallback"

[[rule_graph.nodes]]
id = "provider"
type = "route_provider"
position = { x = 360.0, y = 0.0 }

[rule_graph.nodes.route_provider]
provider_id = "kimi"
"#,
            r#"
[[rule_graph.edges]]
id = "edge-plugin-code"
source = "plugin"
source_handle = "code"
target = "code-header"

[[rule_graph.edges]]
id = "edge-plugin-default"
source = "plugin"
source_handle = "default"
target = "fallback-header"

[[rule_graph.edges]]
id = "edge-code-provider"
source = "code-header"
target = "provider"

[[rule_graph.edges]]
id = "edge-fallback-provider"
source = "fallback-header"
target = "provider"

[[rule_graph.edges]]
id = "edge-provider-end"
source = "provider"
target = "end"
"#,
        );
        let registry = load_test_registry(&["code", "default"], &[]);
        let runtime = FakePluginRuntime::succeeds(RuntimeExecuteOutput {
            context_ops: vec![RuntimeContextPatchOp::Set {
                key: "intent".to_string(),
                value: "code".to_string(),
            }],
            next_port: Some("code".to_string()),
            ..RuntimeExecuteOutput::default()
        });

        let resolution = execute_test_graph(&config, &registry, &runtime, "/v1/chat/completions")
            .expect("graph execution should succeed");

        assert_eq!(resolution.provider.id, "kimi");
        assert_eq!(header_values(&resolution, "x-intent"), vec!["code"]);
        assert_eq!(runtime.calls(), 1);
    }

    #[test]
    fn applies_path_rewrite_from_wasm_plugin_output() {
        let config = parse_graph_config(
            "",
            r#"
[[rule_graph.nodes]]
id = "provider"
type = "route_provider"
position = { x = 240.0, y = 0.0 }

[rule_graph.nodes.route_provider]
provider_id = "kimi"
"#,
            r#"
[[rule_graph.edges]]
id = "edge-plugin-provider"
source = "plugin"
target = "provider"

[[rule_graph.edges]]
id = "edge-provider-end"
source = "provider"
target = "end"
"#,
        );
        let registry = load_test_registry(&["default"], &[]);
        let runtime = FakePluginRuntime::succeeds(RuntimeExecuteOutput {
            path_rewrite: Some("/coding/v1/chat/completions".to_string()),
            ..RuntimeExecuteOutput::default()
        });

        let resolution = execute_test_graph(&config, &registry, &runtime, "/v1/chat/completions")
            .expect("graph execution should succeed");

        assert_eq!(resolution.path, "/coding/v1/chat/completions");
        assert_eq!(runtime.calls(), 1);
    }

    #[test]
    fn rejects_a_denied_capability_grant() {
        let config = parse_graph_config(
            "",
            r#"
[[rule_graph.nodes]]
id = "provider"
type = "route_provider"
position = { x = 240.0, y = 0.0 }

[rule_graph.nodes.route_provider]
provider_id = "kimi"
"#,
            r#"
[[rule_graph.edges]]
id = "edge-plugin-provider"
source = "plugin"
target = "provider"

[[rule_graph.edges]]
id = "edge-provider-end"
source = "provider"
target = "end"
"#,
        );
        let registry = load_test_registry(&["default"], &["network"]);
        let runtime = FakePluginRuntime::succeeds(RuntimeExecuteOutput::default());

        let error = execute_test_graph(&config, &registry, &runtime, "/v1/chat/completions")
            .expect_err("missing grant should fail closed");

        assert!(
            error.contains("requires capability 'network'"),
            "unexpected error: {error}"
        );
        assert_eq!(runtime.calls(), 0);
    }

    #[test]
    fn rejects_granted_capability_not_declared_by_manifest() {
        let config = parse_graph_config(
            "granted_capabilities = [\"network\"]\nallowed_hosts = [\"127.0.0.1:443\"]",
            r#"
[[rule_graph.nodes]]
id = "provider"
type = "route_provider"
position = { x = 240.0, y = 0.0 }

[rule_graph.nodes.route_provider]
provider_id = "kimi"
"#,
            r#"
[[rule_graph.edges]]
id = "edge-plugin-provider"
source = "plugin"
target = "provider"

[[rule_graph.edges]]
id = "edge-provider-end"
source = "provider"
target = "end"
"#,
        );
        let registry = load_test_registry(&["default"], &[]);
        let runtime = FakePluginRuntime::succeeds(RuntimeExecuteOutput::default());

        let error = execute_test_graph(&config, &registry, &runtime, "/v1/chat/completions")
            .expect_err("undeclared capability grants should fail");

        assert!(
            error.contains("does not declare capability 'network'"),
            "unexpected error: {error}"
        );
        assert_eq!(runtime.calls(), 0);
    }

    #[test]
    fn resolves_preopened_dirs_relative_to_workspace_root() {
        let registry_root = temp_dir("preopens");
        write_plugin(&registry_root, "intent-classifier", &["default"], &["fs"]);
        let plugin = load_plugin_registry(&registry_root)
            .expect("plugin registry should load")
            .get("intent-classifier")
            .expect("plugin should exist")
            .clone();
        let workspace_root = plugin_workspace_root(&plugin);
        let node_config = WasmPluginNodeConfig {
            plugin_id: "intent-classifier".to_string(),
            timeout_ms: 20,
            fuel: None,
            max_memory_bytes: 16 * 1024 * 1024,
            granted_capabilities: vec![WasmCapability::Fs],
            read_dirs: vec!["plugins-data/common".to_string()],
            write_dirs: vec!["plugins-data/runtime".to_string()],
            allowed_hosts: Vec::new(),
            config: toml::value::Table::new(),
        };

        let preopens =
            resolve_plugin_preopens(&plugin, &node_config).expect("preopens should resolve");

        assert_eq!(preopens.len(), 2);
        assert_eq!(preopens[0].guest_path, "/plugins-data/common");
        assert_eq!(
            preopens[0].host_path,
            workspace_root.join("plugins-data/common")
        );
        assert!(preopens[0].dir_perms.contains(DirPerms::READ));
        assert!(!preopens[0].dir_perms.contains(DirPerms::MUTATE));
        assert!(preopens[0].file_perms.contains(FilePerms::READ));
        assert!(!preopens[0].file_perms.contains(FilePerms::WRITE));
        assert_eq!(preopens[1].guest_path, "/plugins-data/runtime");
        assert_eq!(
            preopens[1].host_path,
            workspace_root.join("plugins-data/runtime")
        );
        assert!(preopens[1].dir_perms.contains(DirPerms::READ));
        assert!(preopens[1].dir_perms.contains(DirPerms::MUTATE));
        assert!(preopens[1].file_perms.contains(FilePerms::READ));
        assert!(preopens[1].file_perms.contains(FilePerms::WRITE));
    }

    #[test]
    fn resolves_network_allowlist_and_denies_bind_addresses() {
        let node_config = WasmPluginNodeConfig {
            plugin_id: "intent-classifier".to_string(),
            timeout_ms: 20,
            fuel: None,
            max_memory_bytes: 16 * 1024 * 1024,
            granted_capabilities: vec![WasmCapability::Network],
            read_dirs: Vec::new(),
            write_dirs: Vec::new(),
            allowed_hosts: vec!["127.0.0.1:443".to_string()],
            config: toml::value::Table::new(),
        };

        let policy = resolve_plugin_network_policy(&node_config)
            .expect("network policy should resolve")
            .expect("network grant should produce a policy");
        let allowed = "127.0.0.1:443"
            .parse::<SocketAddr>()
            .expect("socket addr should parse");
        let denied = "127.0.0.1:8443"
            .parse::<SocketAddr>()
            .expect("socket addr should parse");

        assert!(!policy.allow_ip_name_lookup);
        assert!(socket_addr_allowed(
            policy.allowed_addrs.as_ref(),
            allowed,
            SocketAddrUse::TcpConnect
        ));
        assert!(!socket_addr_allowed(
            policy.allowed_addrs.as_ref(),
            denied,
            SocketAddrUse::TcpConnect
        ));
        assert!(!socket_addr_allowed(
            policy.allowed_addrs.as_ref(),
            allowed,
            SocketAddrUse::TcpBind
        ));
    }

    #[test]
    fn fails_cleanly_when_plugin_returns_an_unknown_port() {
        let config = parse_graph_config(
            "",
            r#"
[[rule_graph.nodes]]
id = "provider"
type = "route_provider"
position = { x = 240.0, y = 0.0 }

[rule_graph.nodes.route_provider]
provider_id = "kimi"
"#,
            r#"
[[rule_graph.edges]]
id = "edge-plugin-provider"
source = "plugin"
source_handle = "default"
target = "provider"

[[rule_graph.edges]]
id = "edge-provider-end"
source = "provider"
target = "end"
"#,
        );
        let registry = load_test_registry(&["default"], &[]);
        let runtime = FakePluginRuntime::succeeds(RuntimeExecuteOutput {
            next_port: Some("unknown".to_string()),
            ..RuntimeExecuteOutput::default()
        });

        let error = execute_test_graph(&config, &registry, &runtime, "/v1/chat/completions")
            .expect_err("unknown next_port should fail");

        assert!(
            error.contains("returned unknown port 'unknown'"),
            "unexpected error: {error}"
        );
        assert_eq!(runtime.calls(), 1);
    }
}
