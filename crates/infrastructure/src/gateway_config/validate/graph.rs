use std::collections::HashSet;

use application::{
    validate_rule_code_runner_node, validate_rule_condition_node, validate_rule_copy_header_node,
    validate_rule_graph_structure, validate_rule_header_mutation_node,
    validate_rule_header_name_node, validate_rule_log_node, validate_rule_match_node,
    validate_rule_route_provider_node, validate_rule_router_node, validate_rule_select_model_node,
    validate_rule_set_context_node, validate_rule_value_node, validate_rule_wasm_plugin_node,
};

use crate::gateway_config::model::{
    MatchNodeConfig, ModelConfig, RouterNodeConfig, RuleGraphConfig, RuleGraphNode,
    RuleGraphNodeType, WasmPluginNodeConfig,
};

use super::mapping::{
    map_condition_mode, map_match_branches, map_router_rules, map_rule_graph_structure_input,
    map_wasm_grants,
};

pub(crate) fn validate_rule_graph(
    graph: &RuleGraphConfig,
    provider_ids: &HashSet<&str>,
    model_ids: &HashSet<&str>,
    models: &[ModelConfig],
) -> Result<(), Box<dyn std::error::Error>> {
    let provider_ids_owned = provider_ids.iter().map(|id| id.to_string()).collect();
    let model_ids_owned = model_ids.iter().map(|id| id.to_string()).collect();

    validate_rule_graph_structure(&map_rule_graph_structure_input(graph))
        .map_err(|error| error.to_string())?;

    for node in &graph.nodes {
        validate_rule_graph_node(node, graph, &provider_ids_owned, &model_ids_owned, models)?;
    }

    Ok(())
}

fn validate_rule_graph_node(
    node: &RuleGraphNode,
    graph: &RuleGraphConfig,
    provider_ids_owned: &HashSet<String>,
    model_ids_owned: &HashSet<String>,
    models: &[ModelConfig],
) -> Result<(), Box<dyn std::error::Error>> {
    match node.node_type {
        RuleGraphNodeType::Start | RuleGraphNodeType::End | RuleGraphNodeType::Note => {}
        RuleGraphNodeType::Condition => validate_condition_node(node, graph)?,
        RuleGraphNodeType::RouteProvider => validate_route_provider_node(node, provider_ids_owned)?,
        RuleGraphNodeType::SelectModel => {
            validate_select_model_node(node, provider_ids_owned, model_ids_owned, models)?
        }
        RuleGraphNodeType::RewritePath => validate_rule_value_node(
            node.id.as_str(),
            node.rewrite_path
                .as_ref()
                .map(|config| config.value.as_str()),
        )
        .map_err(|error| error.to_string())?,
        RuleGraphNodeType::SetContext => validate_rule_set_context_node(
            node.id.as_str(),
            node.set_context.as_ref().map(|config| config.key.as_str()),
            node.set_context
                .as_ref()
                .map(|config| config.value_template.as_str()),
        )
        .map_err(|error| error.to_string())?,
        RuleGraphNodeType::Router => {
            validate_router_node(node.id.as_str(), graph, node.router.as_ref())?
        }
        RuleGraphNodeType::Log => validate_rule_log_node(
            node.id.as_str(),
            node.log.as_ref().map(|config| config.message.as_str()),
        )
        .map_err(|error| error.to_string())?,
        RuleGraphNodeType::SetHeader => validate_rule_header_mutation_node(
            node.id.as_str(),
            node.set_header.as_ref().map(|config| config.name.as_str()),
            node.set_header.as_ref().map(|config| config.value.as_str()),
        )
        .map_err(|error| error.to_string())?,
        RuleGraphNodeType::RemoveHeader => validate_rule_header_name_node(
            node.id.as_str(),
            node.remove_header
                .as_ref()
                .map(|config| config.name.as_str()),
        )
        .map_err(|error| error.to_string())?,
        RuleGraphNodeType::CopyHeader => validate_rule_copy_header_node(
            node.id.as_str(),
            node.copy_header.as_ref().map(|config| config.from.as_str()),
            node.copy_header.as_ref().map(|config| config.to.as_str()),
        )
        .map_err(|error| error.to_string())?,
        RuleGraphNodeType::SetHeaderIfAbsent => validate_rule_header_mutation_node(
            node.id.as_str(),
            node.set_header_if_absent
                .as_ref()
                .map(|config| config.name.as_str()),
            node.set_header_if_absent
                .as_ref()
                .map(|config| config.value.as_str()),
        )
        .map_err(|error| error.to_string())?,
        RuleGraphNodeType::WasmPlugin => {
            validate_wasm_plugin_node(node.id.as_str(), node.wasm_plugin.as_ref())?
        }
        RuleGraphNodeType::Match => {
            validate_match_node(node.id.as_str(), graph, node.match_node.as_ref())?
        }
        RuleGraphNodeType::CodeRunner => validate_rule_code_runner_node(
            node.id.as_str(),
            node.code_runner.as_ref().map(|config| config.timeout_ms),
            node.code_runner
                .as_ref()
                .map(|config| config.max_memory_bytes),
            node.code_runner.as_ref().map(|config| config.code.as_str()),
        )
        .map_err(|error| error.to_string())?,
    }
    Ok(())
}

fn validate_condition_node(
    node: &RuleGraphNode,
    graph: &RuleGraphConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(condition) = &node.condition else {
        return Err(format!("rule_graph node '{}' missing condition config", node.id).into());
    };
    let outgoing = graph
        .edges
        .iter()
        .filter(|edge| edge.source == node.id)
        .count();
    validate_rule_condition_node(
        node.id.as_str(),
        map_condition_mode(condition.mode),
        condition.expression.as_deref(),
        condition.builder.as_ref().map(|builder| {
            (
                builder.field.as_str(),
                builder.operator.as_str(),
                builder.value.as_str(),
            )
        }),
        outgoing,
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

fn validate_route_provider_node(
    node: &RuleGraphNode,
    provider_ids_owned: &HashSet<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_rule_route_provider_node(
        node.id.as_str(),
        node.route_provider
            .as_ref()
            .map(|config| config.provider_id.as_str()),
        provider_ids_owned,
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

fn validate_select_model_node(
    node: &RuleGraphNode,
    provider_ids_owned: &HashSet<String>,
    model_ids_owned: &HashSet<String>,
    models: &[ModelConfig],
) -> Result<(), Box<dyn std::error::Error>> {
    let model_provider_id = node.select_model.as_ref().and_then(|config| {
        models
            .iter()
            .find(|model| model.id == config.model_id)
            .map(|model| model.provider_id.as_str())
    });
    validate_rule_select_model_node(
        node.id.as_str(),
        node.select_model
            .as_ref()
            .map(|config| config.provider_id.as_str()),
        node.select_model
            .as_ref()
            .map(|config| config.model_id.as_str()),
        provider_ids_owned,
        model_ids_owned,
        model_provider_id,
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

fn validate_router_node(
    node_id: &str,
    graph: &RuleGraphConfig,
    config: Option<&RouterNodeConfig>,
) -> Result<(), Box<dyn std::error::Error>> {
    let node_ids = graph
        .nodes
        .iter()
        .map(|node| node.id.clone())
        .collect::<HashSet<_>>();
    let rules = map_router_rules(config);
    let fallback = config.and_then(|config| config.fallback_node_id.as_deref());

    validate_rule_router_node(node_id, &node_ids, rules.as_deref(), fallback)
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn validate_wasm_plugin_node(
    node_id: &str,
    config: Option<&WasmPluginNodeConfig>,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(config) = config else {
        return validate_rule_wasm_plugin_node(node_id, None, None, None, None, &[], &[], &[], &[])
            .map_err(|error| error.to_string())
            .map_err(Into::into);
    };
    let granted_capabilities = map_wasm_grants(config);
    validate_rule_wasm_plugin_node(
        node_id,
        Some(config.plugin_id.as_str()),
        Some(config.timeout_ms),
        Some(config.fuel),
        Some(config.max_memory_bytes),
        &granted_capabilities,
        &config.read_dirs,
        &config.write_dirs,
        &config.allowed_hosts,
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

fn validate_match_node(
    node_id: &str,
    graph: &RuleGraphConfig,
    config: Option<&MatchNodeConfig>,
) -> Result<(), Box<dyn std::error::Error>> {
    let node_ids = graph
        .nodes
        .iter()
        .map(|node| node.id.clone())
        .collect::<HashSet<_>>();
    let Some(config) = config else {
        validate_rule_match_node(node_id, &node_ids, None, None)
            .map_err(|error| error.to_string())?;
        return Ok(());
    };
    validate_wasm_plugin_node(node_id, Some(&config.plugin))?;
    let branches = map_match_branches(config);
    validate_rule_match_node(
        node_id,
        &node_ids,
        Some(&branches),
        config.fallback_node_id.as_deref(),
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}
