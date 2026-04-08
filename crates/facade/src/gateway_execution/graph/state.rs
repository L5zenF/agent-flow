use std::collections::{HashMap, HashSet};

use axum::http::{HeaderMap, HeaderName, HeaderValue, Method, Uri};
use infrastructure::plugin_registry::PluginRegistry;

use crate::config::{GatewayConfig, ModelConfig, ProviderConfig, RuleGraphConfig, RuleGraphNode};
use crate::gateway_execution::context::{
    SELECTED_MODEL_CONTEXT_KEY, SELECTED_PROVIDER_CONTEXT_KEY, inject_runtime_context,
    resolve_provider_header_for_graph, sync_selected_targets_from_context,
};
use crate::rules::RequestContext;

use super::super::RequestResolution;
use super::GraphNodeExecutor;

pub(super) struct GraphExecutionState<'cfg, 'env> {
    pub(super) config: &'cfg GatewayConfig,
    pub(super) plugin_registry: &'env PluginRegistry,
    pub(super) executor: &'env dyn GraphNodeExecutor,
    pub(super) graph: &'env RuleGraphConfig,
    pub(super) method: &'env Method,
    pub(super) headers: &'env HeaderMap,
    pub(super) fallback_route: Option<&'cfg crate::config::RouteConfig>,
    pub(super) node_map: HashMap<&'env str, &'env RuleGraphNode>,
    pub(super) current_id: &'env str,
    pub(super) selected_provider: Option<&'cfg ProviderConfig>,
    pub(super) selected_model: Option<&'cfg ModelConfig>,
    pub(super) resolved_path: String,
    pub(super) workflow_context: HashMap<String, String>,
    pub(super) outgoing_headers: HashMap<String, Vec<String>>,
    traversed: HashSet<String>,
    max_steps: usize,
}

impl<'cfg, 'env> GraphExecutionState<'cfg, 'env> {
    pub(super) fn new(
        config: &'cfg GatewayConfig,
        plugin_registry: &'env PluginRegistry,
        executor: &'env dyn GraphNodeExecutor,
        graph: &'env RuleGraphConfig,
        method: &'env Method,
        uri: &Uri,
        headers: &'env HeaderMap,
    ) -> Result<Self, String> {
        let node_map = graph
            .nodes
            .iter()
            .map(|node| (node.id.as_str(), node))
            .collect::<HashMap<_, _>>();

        Ok(Self {
            config,
            plugin_registry,
            executor,
            graph,
            method,
            headers,
            fallback_route: config.routes.first(),
            node_map,
            current_id: graph.start_node_id.as_str(),
            selected_provider: None,
            selected_model: None,
            resolved_path: uri.path().to_string(),
            workflow_context: HashMap::new(),
            outgoing_headers: HashMap::new(),
            traversed: HashSet::new(),
            max_steps: graph.nodes.len().saturating_mul(3).max(1),
        })
    }

    pub(super) fn run(mut self) -> Result<RequestResolution<'cfg>, String> {
        for _ in 0..self.max_steps {
            let node = self.current_node()?;
            self.traversed.insert(self.current_id.to_string());
            self.inject_runtime_context();
            let next = self.execute_node(node)?;
            self.sync_selected_targets()?;
            self.current_id = match next {
                Some(next_id) => next_id,
                None => break,
            };
        }

        if self.traversed.len() >= self.max_steps {
            return Err("rule_graph exceeded maximum execution steps".to_string());
        }

        self.into_resolution()
    }

    fn current_node(&self) -> Result<&'env RuleGraphNode, String> {
        self.node_map
            .get(self.current_id)
            .copied()
            .ok_or_else(|| format!("rule_graph node '{}' does not exist", self.current_id))
    }

    fn inject_runtime_context(&mut self) {
        inject_runtime_context(
            &mut self.workflow_context,
            self.method.as_str(),
            &self.resolved_path,
            self.headers,
            self.selected_provider,
            self.selected_model,
            self.fallback_route,
        );
    }

    pub(super) fn request_context(&self) -> RequestContext<'_> {
        RequestContext {
            method: self.method.as_str(),
            path: &self.resolved_path,
            headers: self.headers,
            context: &self.workflow_context,
            provider: self.selected_provider,
            model: self.selected_model,
            route: self.fallback_route,
        }
    }

    pub(super) fn sync_selected_targets(&mut self) -> Result<(), String> {
        sync_selected_targets_from_context(
            self.config,
            &self.workflow_context,
            &mut self.outgoing_headers,
            &mut self.selected_provider,
            &mut self.selected_model,
            SELECTED_PROVIDER_CONTEXT_KEY,
            SELECTED_MODEL_CONTEXT_KEY,
        )
    }

    pub(super) fn apply_selected_provider_headers(&mut self) -> Result<(), String> {
        if let Some(provider) = self.selected_provider {
            for header in &provider.default_headers {
                let value = resolve_provider_header_for_graph(header, self.config)?;
                self.outgoing_headers
                    .insert(header.name.to_ascii_lowercase(), vec![value]);
            }
        }
        Ok(())
    }

    pub(super) fn find_provider(&self, provider_id: &str) -> Result<&'cfg ProviderConfig, String> {
        self.config
            .providers
            .iter()
            .find(|provider| provider.id == provider_id)
            .ok_or_else(|| format!("rule_graph provider '{}' not found", provider_id))
    }

    pub(super) fn find_model(&self, model_id: &str) -> Result<&'cfg ModelConfig, String> {
        self.config
            .models
            .iter()
            .find(|model| model.id == model_id)
            .ok_or_else(|| format!("rule_graph model '{}' not found", model_id))
    }

    pub(super) fn resolve_runtime_port(
        &self,
        node_id: &str,
        plugin_id: &str,
        port: Option<String>,
    ) -> Result<Option<&'env str>, String> {
        match port.as_deref() {
            Some(port) => {
                let plugin = self
                    .plugin_registry
                    .get(plugin_id)
                    .expect("plugin should exist after successful execution");
                super::validate_plugin_port(plugin, port, node_id)?;
                Ok(Some(
                    crate::gateway_execution::context::next_condition_edge(
                        self.graph,
                        self.current_id,
                        port,
                    )?
                    .ok_or_else(|| {
                        format!(
                            "plugin '{}' returned port '{}' for node '{}' but no outgoing edge matched it",
                            plugin.plugin_id(),
                            port,
                            node_id
                        )
                    })?,
                ))
            }
            None => Ok(crate::gateway_execution::context::next_condition_edge(
                self.graph,
                self.current_id,
                "default",
            )?
            .or(crate::gateway_execution::context::next_linear_edge(
                self.graph,
                self.current_id,
            )?)),
        }
    }

    pub(super) fn resolve_code_runner_port(
        &self,
        node_id: &str,
        port: Option<String>,
    ) -> Result<Option<&'env str>, String> {
        match port.as_deref() {
            Some(port) => Ok(Some(
                crate::gateway_execution::context::next_condition_edge(
                    self.graph,
                    self.current_id,
                    port,
                )?
                .ok_or_else(|| {
                    format!(
                        "code_runner node '{}' returned port '{}' but no outgoing edge matched it",
                        node_id, port
                    )
                })?,
            )),
            None => Ok(crate::gateway_execution::context::next_condition_edge(
                self.graph,
                self.current_id,
                "default",
            )?
            .or(crate::gateway_execution::context::next_linear_edge(
                self.graph,
                self.current_id,
            )?)),
        }
    }

    fn into_resolution(self) -> Result<RequestResolution<'cfg>, String> {
        let provider = self
            .selected_provider
            .ok_or_else(|| "rule_graph did not select a provider".to_string())?;
        let mut extra_headers = Vec::new();
        for (name, values) in self.outgoing_headers {
            let header_name = HeaderName::try_from(name).map_err(|error| error.to_string())?;
            for value in values {
                let header_value =
                    HeaderValue::from_str(&value).map_err(|error| error.to_string())?;
                extra_headers.push((header_name.clone(), header_value));
            }
        }

        Ok(RequestResolution {
            provider,
            model: self.selected_model,
            path: self.resolved_path,
            extra_headers,
            route: None,
        })
    }
}
