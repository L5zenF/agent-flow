use std::collections::HashMap;

use axum::http::{HeaderMap, HeaderName, HeaderValue, Method, Uri};
use infrastructure::plugin_registry::{LoadedPlugin, PluginRegistry};

use crate::config::{
    CodeRunnerNodeConfig, ConditionMode, GatewayConfig, ModelConfig, ProviderConfig,
    RuleGraphConfig, RuleGraphNodeType, WasmPluginNodeConfig,
};
use crate::gateway_execution::context::{
    inject_runtime_context, next_condition_edge, next_linear_edge, resolve_provider_header_for_graph,
    sync_selected_targets_from_context, SELECTED_MODEL_CONTEXT_KEY,
    SELECTED_PROVIDER_CONTEXT_KEY,
};
use crate::gateway_execution::resolve::RequestResolution;
use crate::gateway_execution::route_match::evaluate_router_clause;
use crate::rules::{evaluate_expression, render_template, RequestContext};

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
        workflow_context: &mut HashMap<String, String>,
        outgoing_headers: &mut HashMap<String, Vec<String>>,
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
        workflow_context: &mut HashMap<String, String>,
        outgoing_headers: &mut HashMap<String, Vec<String>>,
    ) -> Result<Option<String>, String>;
}

pub fn execute_rule_graph<'a>(
    config: &'a GatewayConfig,
    plugin_registry: &PluginRegistry,
    executor: &dyn GraphNodeExecutor,
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
    let mut traversed = std::collections::HashSet::<String>::new();
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
                        format!("{} {} \"{}\"", builder.field, builder.operator, builder.value)
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
                        format!("rule_graph node '{}' missing route_provider config", node.id)
                    })?
                    .provider_id
                    .as_str();
                selected_provider = Some(
                    config
                        .providers
                        .iter()
                        .find(|provider| provider.id == provider_id)
                        .ok_or_else(|| format!("rule_graph provider '{}' not found", provider_id))?,
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
                    config.providers.iter().find(|provider| provider.id == provider_id).ok_or_else(|| {
                        format!("rule_graph provider '{}' not found", provider_id)
                    })?,
                );
                selected_model = Some(
                    config.models.iter().find(|model| model.id == model_id).ok_or_else(|| {
                        format!("rule_graph model '{}' not found", model_id)
                    })?,
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
                let port = executor.execute_wasm_runtime_node(
                    node.id.as_str(),
                    node_config,
                    plugin_registry,
                    method.as_str(),
                    headers,
                    selected_provider,
                    selected_model,
                    &mut resolved_path,
                    &mut workflow_context,
                    &mut outgoing_headers,
                )?;
                match port.as_deref() {
                    Some(port) => {
                        let plugin = plugin_registry
                            .get(node_config.plugin_id.as_str())
                            .expect("plugin should exist after successful execution");
                        validate_plugin_port(plugin, port, node.id.as_str())?;
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
            RuleGraphNodeType::Match => {
                let node_config = node
                    .match_node
                    .as_ref()
                    .ok_or_else(|| format!("rule_graph node '{}' missing match config", node.id))?;
                let mut matched_target = None;
                for branch in &node_config.branches {
                    let mut branch_plugin_config = node_config.plugin.clone();
                    branch_plugin_config
                        .config
                        .insert("expr".to_string(), toml::Value::String(branch.expr.clone()));
                    branch_plugin_config.config.insert(
                        "branch_id".to_string(),
                        toml::Value::String(branch.id.clone()),
                    );
                    let port = executor.execute_wasm_runtime_node(
                        node.id.as_str(),
                        &branch_plugin_config,
                        plugin_registry,
                        method.as_str(),
                        headers,
                        selected_provider,
                        selected_model,
                        &mut resolved_path,
                        &mut workflow_context,
                        &mut outgoing_headers,
                    )?;
                    if matches!(port.as_deref(), Some("match")) {
                        matched_target = Some(branch.target_node_id.as_str());
                        break;
                    }
                }
                matched_target.or(node_config.fallback_node_id.as_deref())
            }
            RuleGraphNodeType::CodeRunner => {
                let port = executor.execute_code_runner_node(
                    plugin_registry,
                    node.id.as_str(),
                    node.code_runner.as_ref().ok_or_else(|| {
                        format!("rule_graph node '{}' missing code_runner config", node.id)
                    })?,
                    method.as_str(),
                    headers,
                    selected_provider,
                    selected_model,
                    &mut resolved_path,
                    &mut workflow_context,
                    &mut outgoing_headers,
                )?;
                match port.as_deref() {
                    Some(port) => Some(next_condition_edge(graph, current_id, port)?.ok_or_else(|| {
                        format!(
                            "code_runner node '{}' returned port '{}' but no outgoing edge matched it",
                            node.id, port
                        )
                    })?),
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
                tracing::info!(node_id = %node.id, message = %message, "rule graph log");
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

        sync_selected_targets_from_context(
            config,
            &workflow_context,
            &mut outgoing_headers,
            &mut selected_provider,
            &mut selected_model,
            SELECTED_PROVIDER_CONTEXT_KEY,
            SELECTED_MODEL_CONTEXT_KEY,
        )?;

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
