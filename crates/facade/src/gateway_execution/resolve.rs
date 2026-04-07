use std::collections::HashMap;

use axum::http::{HeaderMap, HeaderName, HeaderValue, Method, Uri};
use infrastructure::plugin_registry::PluginRegistry;

use crate::config::{GatewayConfig, LoadedWorkflowSet, ModelConfig, ProviderConfig, RouteConfig};
use crate::gateway_execution::{execute_rule_graph, inject_runtime_context, resolve_route, GraphNodeExecutor};
use crate::rules::{build_header_map, RequestContext};

#[derive(Debug)]
pub struct RequestResolution<'a> {
    pub provider: &'a ProviderConfig,
    pub model: Option<&'a ModelConfig>,
    pub path: String,
    pub extra_headers: Vec<(HeaderName, HeaderValue)>,
    pub route: Option<&'a RouteConfig>,
}

pub fn resolve_request<'a>(
    config: &'a GatewayConfig,
    workflow_store: &'a LoadedWorkflowSet,
    plugin_registry: &PluginRegistry,
    executor: &dyn GraphNodeExecutor,
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
            return execute_rule_graph(config, plugin_registry, executor, graph, method, uri, headers)
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
