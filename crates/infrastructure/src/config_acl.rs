use std::collections::HashSet;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::path::{Component, Path};

use domain::{
    DomainError, GatewayCatalog, Model, ModelCatalog, ModelId, Provider, ProviderCatalog,
    ProviderId, RouteId, Workflow, WorkflowId, WorkflowIndex,
};
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RawProvider {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RawModel {
    pub id: String,
    pub name: String,
    pub provider_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RawRoute {
    pub id: String,
    #[serde(alias = "match")]
    pub matcher: String,
    pub provider_id: String,
    #[serde(default)]
    pub model_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawRuleScope {
    Global,
    Provider,
    Model,
    Route,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RawHeaderRule {
    pub id: String,
    pub scope: RawRuleScope,
    #[serde(default)]
    pub target_id: Option<String>,
    #[serde(default)]
    pub actions: Vec<toml::Value>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct RawRuleGraphConfig {
    #[serde(default)]
    pub start_node_id: String,
    #[serde(default)]
    pub nodes: Vec<RawRuleGraphNode>,
    #[serde(default)]
    pub edges: Vec<RawRuleGraphEdge>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RawRuleGraphNode {
    pub id: String,
    #[serde(rename = "type")]
    pub node_type: RawRuleGraphNodeType,
    #[serde(default)]
    pub condition: Option<RawConditionNodeConfig>,
    #[serde(default)]
    pub log: Option<RawLogNodeConfig>,
    #[serde(default)]
    pub set_header: Option<RawHeaderMutationNodeConfig>,
    #[serde(default)]
    pub remove_header: Option<RawHeaderNameNodeConfig>,
    #[serde(default)]
    pub copy_header: Option<RawCopyHeaderNodeConfig>,
    #[serde(default)]
    pub select_model: Option<RawSelectModelNodeConfig>,
    #[serde(default)]
    pub router: Option<RawRouterNodeConfig>,
    #[serde(default)]
    pub wasm_plugin: Option<RawWasmPluginNodeConfig>,
    #[serde(default, rename = "match", alias = "wasm_match")]
    pub match_node: Option<RawMatchNodeConfig>,
    #[serde(default)]
    pub code_runner: Option<RawCodeRunnerNodeConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RawRuleGraphEdge {
    pub id: String,
    pub source: String,
    pub target: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawRuleGraphNodeType {
    Start,
    Condition,
    Log,
    SetHeader,
    RemoveHeader,
    CopyHeader,
    SetHeaderIfAbsent,
    SelectModel,
    Router,
    WasmPlugin,
    Match,
    CodeRunner,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawWasmCapability {
    Log,
    Fs,
    Network,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RawWasmPluginNodeConfig {
    pub plugin_id: String,
    #[serde(default = "default_wasm_plugin_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default)]
    pub fuel: Option<u64>,
    pub max_memory_bytes: u64,
    #[serde(default)]
    pub granted_capabilities: Vec<RawWasmCapability>,
    #[serde(default)]
    pub read_dirs: Vec<String>,
    #[serde(default)]
    pub write_dirs: Vec<String>,
    #[serde(default)]
    pub allowed_hosts: Vec<String>,
    #[serde(default)]
    pub config: toml::value::Table,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RawMatchNodeConfig {
    #[serde(flatten)]
    pub plugin: RawWasmPluginNodeConfig,
    #[serde(default)]
    pub branches: Vec<RawMatchBranchConfig>,
    #[serde(default)]
    pub fallback_node_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RawMatchBranchConfig {
    pub id: String,
    pub expr: String,
    pub target_node_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RawCodeRunnerNodeConfig {
    #[serde(default = "default_wasm_plugin_timeout_ms")]
    pub timeout_ms: u64,
    pub max_memory_bytes: u64,
    pub code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RawLogNodeConfig {
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RawHeaderMutationNodeConfig {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RawHeaderNameNodeConfig {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RawCopyHeaderNodeConfig {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawConditionMode {
    Builder,
    Expression,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RawConditionNodeConfig {
    pub mode: RawConditionMode,
    #[serde(default)]
    pub expression: Option<String>,
    #[serde(default)]
    pub builder: Option<RawConditionBuilderConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RawConditionBuilderConfig {
    pub field: String,
    pub operator: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RawSelectModelNodeConfig {
    pub provider_id: String,
    pub model_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RawRouterNodeConfig {
    #[serde(default)]
    pub rules: Vec<RawRouterRule>,
    #[serde(default)]
    pub fallback_node_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RawRouterRule {
    pub id: String,
    #[serde(default)]
    pub clauses: Vec<RawRouterClause>,
    pub target_node_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RawRouterClause {
    pub source: String,
    pub operator: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RawWorkflowIndexEntry {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub file: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct RawWorkflowIndex {
    #[serde(default)]
    pub workflows: Vec<RawWorkflowIndexEntry>,
    #[serde(default)]
    pub active_workflow_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct RawGatewayConfig {
    #[serde(default)]
    pub providers: Vec<RawProvider>,
    #[serde(default)]
    pub models: Vec<RawModel>,
    #[serde(default)]
    pub routes: Vec<RawRoute>,
    #[serde(default)]
    pub header_rules: Vec<RawHeaderRule>,
    #[serde(default)]
    pub workflows: Vec<RawWorkflowIndexEntry>,
    #[serde(default)]
    pub active_workflow_id: Option<String>,
    #[serde(default)]
    pub workflows_dir: Option<String>,
    #[serde(default)]
    pub rule_graph: Option<RawRuleGraphConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InfrastructureAclError {
    Domain(DomainError),
    TomlDeserialize(String),
    Validation(String),
}

impl Display for InfrastructureAclError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Domain(error) => error.fmt(formatter),
            Self::TomlDeserialize(error) => formatter.write_str(error),
            Self::Validation(error) => formatter.write_str(error),
        }
    }
}

impl Error for InfrastructureAclError {}

impl From<DomainError> for InfrastructureAclError {
    fn from(value: DomainError) -> Self {
        Self::Domain(value)
    }
}

pub fn parse_raw_gateway_config(raw: &str) -> Result<RawGatewayConfig, InfrastructureAclError> {
    toml::from_str(raw).map_err(|error| InfrastructureAclError::TomlDeserialize(error.to_string()))
}

pub fn map_gateway_config(
    raw: &RawGatewayConfig,
) -> Result<(GatewayCatalog, WorkflowIndex), InfrastructureAclError> {
    validate_workflow_index_subset(raw)?;
    validate_route_and_header_rule_subset(raw)?;
    if let Some(graph) = raw.rule_graph.as_ref() {
        validate_rule_graph_subset(graph, raw)?;
    }
    let gateway_catalog = map_gateway_catalog(&raw.providers, &raw.models)?;
    let workflow_index = map_workflow_index(&RawWorkflowIndex {
        workflows: raw.workflows.clone(),
        active_workflow_id: raw.active_workflow_id.clone(),
    })?;
    Ok((gateway_catalog, workflow_index))
}

fn validate_rule_graph_subset(
    graph: &RawRuleGraphConfig,
    raw: &RawGatewayConfig,
) -> Result<(), InfrastructureAclError> {
    if graph.start_node_id.trim().is_empty() {
        return Err(InfrastructureAclError::Validation(
            "rule_graph start_node_id cannot be empty".to_string(),
        ));
    }

    let node_ids = unique_ids(
        graph.nodes.iter().map(|node| node.id.as_str()),
        "rule_graph node",
    )?;
    unique_ids(
        graph.edges.iter().map(|edge| edge.id.as_str()),
        "rule_graph edge",
    )?;

    if !node_ids.contains(graph.start_node_id.as_str()) {
        return Err(InfrastructureAclError::Validation(format!(
            "rule_graph start node '{}' does not exist",
            graph.start_node_id
        )));
    }

    let start_count = graph
        .nodes
        .iter()
        .filter(|node| node.node_type == RawRuleGraphNodeType::Start)
        .count();
    if start_count != 1 {
        return Err(InfrastructureAclError::Validation(format!(
            "rule_graph requires exactly one start node, found {start_count}"
        )));
    }

    for edge in &graph.edges {
        if !node_ids.contains(edge.source.as_str()) {
            return Err(InfrastructureAclError::Validation(format!(
                "rule_graph edge '{}' missing source '{}'",
                edge.id, edge.source
            )));
        }
        if !node_ids.contains(edge.target.as_str()) {
            return Err(InfrastructureAclError::Validation(format!(
                "rule_graph edge '{}' missing target '{}'",
                edge.id, edge.target
            )));
        }
    }

    let provider_ids = raw
        .providers
        .iter()
        .map(|provider| provider.id.as_str())
        .collect::<HashSet<_>>();
    let model_ids = raw
        .models
        .iter()
        .map(|model| model.id.as_str())
        .collect::<HashSet<_>>();

    for node in &graph.nodes {
        match node.node_type {
            RawRuleGraphNodeType::Condition => {
                let Some(condition) = &node.condition else {
                    return Err(InfrastructureAclError::Validation(format!(
                        "rule_graph node '{}' missing condition config",
                        node.id
                    )));
                };
                match condition.mode {
                    RawConditionMode::Expression => {
                        if condition
                            .expression
                            .as_deref()
                            .unwrap_or("")
                            .trim()
                            .is_empty()
                        {
                            return Err(InfrastructureAclError::Validation(format!(
                                "rule_graph condition node '{}' requires expression",
                                node.id
                            )));
                        }
                    }
                    RawConditionMode::Builder => {
                        let Some(builder) = &condition.builder else {
                            return Err(InfrastructureAclError::Validation(format!(
                                "rule_graph condition node '{}' requires builder config",
                                node.id
                            )));
                        };
                        if builder.field.trim().is_empty()
                            || builder.operator.trim().is_empty()
                            || builder.value.trim().is_empty()
                        {
                            return Err(InfrastructureAclError::Validation(format!(
                                "rule_graph condition node '{}' builder fields cannot be empty",
                                node.id
                            )));
                        }
                    }
                }
                let outgoing = graph
                    .edges
                    .iter()
                    .filter(|edge| edge.source == node.id)
                    .count();
                if outgoing > 2 {
                    return Err(InfrastructureAclError::Validation(format!(
                        "rule_graph condition node '{}' supports at most 2 outgoing edges",
                        node.id
                    )));
                }
            }
            RawRuleGraphNodeType::SelectModel => {
                let Some(config) = &node.select_model else {
                    return Err(InfrastructureAclError::Validation(format!(
                        "rule_graph node '{}' missing select_model config",
                        node.id
                    )));
                };
                if !provider_ids.contains(config.provider_id.as_str()) {
                    return Err(InfrastructureAclError::Validation(format!(
                        "rule_graph node '{}' references missing provider '{}'",
                        node.id, config.provider_id
                    )));
                }
                if !model_ids.contains(config.model_id.as_str()) {
                    return Err(InfrastructureAclError::Validation(format!(
                        "rule_graph node '{}' references missing model '{}'",
                        node.id, config.model_id
                    )));
                }
                let Some(model) = raw.models.iter().find(|model| model.id == config.model_id)
                else {
                    return Err(InfrastructureAclError::Validation(format!(
                        "rule_graph node '{}' references missing model '{}'",
                        node.id, config.model_id
                    )));
                };
                if model.provider_id != config.provider_id {
                    return Err(InfrastructureAclError::Validation(format!(
                        "rule_graph node '{}' model '{}' does not belong to provider '{}'",
                        node.id, config.model_id, config.provider_id
                    )));
                }
            }
            RawRuleGraphNodeType::Log => {
                let Some(config) = &node.log else {
                    return Err(InfrastructureAclError::Validation(format!(
                        "rule_graph node '{}' missing log config",
                        node.id
                    )));
                };
                if config.message.trim().is_empty() {
                    return Err(InfrastructureAclError::Validation(format!(
                        "rule_graph node '{}' log message cannot be empty",
                        node.id
                    )));
                }
            }
            RawRuleGraphNodeType::SetHeader | RawRuleGraphNodeType::SetHeaderIfAbsent => {
                let Some(config) = &node.set_header else {
                    return Err(InfrastructureAclError::Validation(format!(
                        "rule_graph node '{}' missing header config",
                        node.id
                    )));
                };
                if config.name.trim().is_empty() || config.value.trim().is_empty() {
                    return Err(InfrastructureAclError::Validation(format!(
                        "rule_graph node '{}' header name/value cannot be empty",
                        node.id
                    )));
                }
            }
            RawRuleGraphNodeType::RemoveHeader => {
                let Some(config) = &node.remove_header else {
                    return Err(InfrastructureAclError::Validation(format!(
                        "rule_graph node '{}' missing remove_header config",
                        node.id
                    )));
                };
                if config.name.trim().is_empty() {
                    return Err(InfrastructureAclError::Validation(format!(
                        "rule_graph node '{}' header name cannot be empty",
                        node.id
                    )));
                }
            }
            RawRuleGraphNodeType::CopyHeader => {
                let Some(config) = &node.copy_header else {
                    return Err(InfrastructureAclError::Validation(format!(
                        "rule_graph node '{}' missing copy_header config",
                        node.id
                    )));
                };
                if config.from.trim().is_empty() || config.to.trim().is_empty() {
                    return Err(InfrastructureAclError::Validation(format!(
                        "rule_graph node '{}' copy header fields cannot be empty",
                        node.id
                    )));
                }
            }
            RawRuleGraphNodeType::Router => {
                let Some(config) = &node.router else {
                    return Err(InfrastructureAclError::Validation(format!(
                        "rule_graph node '{}' missing router config",
                        node.id
                    )));
                };
                if config.rules.is_empty() {
                    return Err(InfrastructureAclError::Validation(format!(
                        "rule_graph node '{}' must define at least one router rule",
                        node.id
                    )));
                }

                let mut rule_ids = HashSet::new();
                for rule in &config.rules {
                    if rule.id.trim().is_empty() {
                        return Err(InfrastructureAclError::Validation(format!(
                            "rule_graph node '{}' contains a router rule with empty id",
                            node.id
                        )));
                    }
                    if !rule_ids.insert(rule.id.as_str()) {
                        return Err(InfrastructureAclError::Validation(format!(
                            "rule_graph node '{}' has duplicate router rule id '{}'",
                            node.id, rule.id
                        )));
                    }
                    if rule.clauses.is_empty() {
                        return Err(InfrastructureAclError::Validation(format!(
                            "rule_graph node '{}' router rule '{}' must contain at least one clause",
                            node.id, rule.id
                        )));
                    }
                    if rule.target_node_id.trim().is_empty()
                        || !node_ids.contains(rule.target_node_id.as_str())
                    {
                        return Err(InfrastructureAclError::Validation(format!(
                            "rule_graph node '{}' router rule '{}' references missing target '{}'",
                            node.id, rule.id, rule.target_node_id
                        )));
                    }
                    for clause in &rule.clauses {
                        if clause.source.trim().is_empty()
                            || clause.operator.trim().is_empty()
                            || clause.value.trim().is_empty()
                        {
                            return Err(InfrastructureAclError::Validation(format!(
                                "rule_graph node '{}' router rule '{}' contains an incomplete clause",
                                node.id, rule.id
                            )));
                        }
                    }
                }

                if let Some(fallback) = config.fallback_node_id.as_deref()
                    && (fallback.trim().is_empty() || !node_ids.contains(fallback))
                {
                    return Err(InfrastructureAclError::Validation(format!(
                        "rule_graph node '{}' references missing fallback target '{}'",
                        node.id, fallback
                    )));
                }
            }
            RawRuleGraphNodeType::WasmPlugin => {
                validate_wasm_plugin_node(node.id.as_str(), node.wasm_plugin.as_ref())?;
            }
            RawRuleGraphNodeType::Match => {
                validate_match_node(node.id.as_str(), graph, node.match_node.as_ref())?;
            }
            RawRuleGraphNodeType::CodeRunner => {
                validate_code_runner_node(node.id.as_str(), node.code_runner.as_ref())?;
            }
            _ => {}
        }
    }

    Ok(())
}

fn validate_route_and_header_rule_subset(
    raw: &RawGatewayConfig,
) -> Result<(), InfrastructureAclError> {
    let provider_ids = raw
        .providers
        .iter()
        .map(|provider| provider.id.as_str())
        .collect::<HashSet<_>>();
    let model_ids = raw
        .models
        .iter()
        .map(|model| model.id.as_str())
        .collect::<HashSet<_>>();
    let route_ids = unique_ids(raw.routes.iter().map(|route| route.id.as_str()), "route")?;
    unique_ids(
        raw.header_rules.iter().map(|rule| rule.id.as_str()),
        "header_rule",
    )?;

    for route in &raw.routes {
        if route.matcher.trim().is_empty() {
            return Err(InfrastructureAclError::Validation(format!(
                "route '{}' matcher cannot be empty",
                route.id
            )));
        }
        if !provider_ids.contains(route.provider_id.as_str()) {
            return Err(InfrastructureAclError::Validation(format!(
                "route '{}' references missing provider '{}'",
                route.id, route.provider_id
            )));
        }
        if let Some(model_id) = route.model_id.as_deref()
            && !model_ids.contains(model_id)
        {
            return Err(InfrastructureAclError::Validation(format!(
                "route '{}' references missing model '{}'",
                route.id, model_id
            )));
        }
    }

    for rule in &raw.header_rules {
        match rule.scope {
            RawRuleScope::Global => {
                if rule.target_id.is_some() {
                    return Err(InfrastructureAclError::Validation(format!(
                        "header_rule '{}' must not define target_id for global scope",
                        rule.id
                    )));
                }
            }
            RawRuleScope::Provider => {
                validate_header_rule_target(rule, &provider_ids, "provider")?;
            }
            RawRuleScope::Model => {
                validate_header_rule_target(rule, &model_ids, "model")?;
            }
            RawRuleScope::Route => {
                validate_header_rule_target(rule, &route_ids, "route")?;
            }
        }

        if rule.actions.is_empty() {
            return Err(InfrastructureAclError::Validation(format!(
                "header_rule '{}' must contain at least one action",
                rule.id
            )));
        }
    }

    Ok(())
}

fn validate_wasm_plugin_node(
    node_id: &str,
    config: Option<&RawWasmPluginNodeConfig>,
) -> Result<(), InfrastructureAclError> {
    let Some(config) = config else {
        return Err(InfrastructureAclError::Validation(format!(
            "rule_graph node '{node_id}' missing wasm_plugin config"
        )));
    };
    validate_wasm_plugin_config(node_id, config)
}

fn validate_wasm_plugin_config(
    node_id: &str,
    config: &RawWasmPluginNodeConfig,
) -> Result<(), InfrastructureAclError> {
    if config.plugin_id.trim().is_empty() {
        return Err(InfrastructureAclError::Validation(format!(
            "rule_graph node '{node_id}' plugin_id cannot be empty"
        )));
    }
    if config.timeout_ms == 0 {
        return Err(InfrastructureAclError::Validation(format!(
            "rule_graph node '{node_id}' timeout_ms must be greater than zero"
        )));
    }
    if matches!(config.fuel, Some(0)) {
        return Err(InfrastructureAclError::Validation(format!(
            "rule_graph node '{node_id}' fuel must be greater than zero when set"
        )));
    }
    if config.max_memory_bytes == 0 {
        return Err(InfrastructureAclError::Validation(format!(
            "rule_graph node '{node_id}' max_memory_bytes must be greater than zero"
        )));
    }

    let grants = config
        .granted_capabilities
        .iter()
        .copied()
        .collect::<HashSet<_>>();
    let has_fs = grants.contains(&RawWasmCapability::Fs);
    let has_network = grants.contains(&RawWasmCapability::Network);

    if has_fs {
        if config.read_dirs.is_empty() && config.write_dirs.is_empty() {
            return Err(InfrastructureAclError::Validation(format!(
                "rule_graph node '{node_id}' fs capability requires read_dirs or write_dirs"
            )));
        }
    } else if !config.read_dirs.is_empty() || !config.write_dirs.is_empty() {
        return Err(InfrastructureAclError::Validation(format!(
            "rule_graph node '{node_id}' fs directories require an fs capability grant"
        )));
    }

    validate_wasm_plugin_paths(node_id, "read_dirs", &config.read_dirs)?;
    validate_wasm_plugin_paths(node_id, "write_dirs", &config.write_dirs)?;

    if has_network {
        if config.allowed_hosts.is_empty() {
            return Err(InfrastructureAclError::Validation(format!(
                "rule_graph node '{node_id}' network capability requires allowed_hosts"
            )));
        }
    } else if !config.allowed_hosts.is_empty() {
        return Err(InfrastructureAclError::Validation(format!(
            "rule_graph node '{node_id}' allowed_hosts require a network capability grant"
        )));
    }

    validate_wasm_plugin_hosts(node_id, &config.allowed_hosts)
}

fn validate_match_node(
    node_id: &str,
    graph: &RawRuleGraphConfig,
    config: Option<&RawMatchNodeConfig>,
) -> Result<(), InfrastructureAclError> {
    let Some(config) = config else {
        return Err(InfrastructureAclError::Validation(format!(
            "rule_graph node '{node_id}' missing match config"
        )));
    };
    validate_wasm_plugin_config(node_id, &config.plugin)?;

    if config.branches.is_empty() {
        return Err(InfrastructureAclError::Validation(format!(
            "rule_graph node '{node_id}' must define at least one match branch"
        )));
    }

    let node_ids = graph
        .nodes
        .iter()
        .map(|node| node.id.as_str())
        .collect::<HashSet<_>>();
    let mut branch_ids = HashSet::new();
    for branch in &config.branches {
        if branch.id.trim().is_empty() {
            return Err(InfrastructureAclError::Validation(format!(
                "rule_graph node '{node_id}' contains a match branch with empty id"
            )));
        }
        if !branch_ids.insert(branch.id.as_str()) {
            return Err(InfrastructureAclError::Validation(format!(
                "rule_graph node '{node_id}' has duplicate match branch id '{}'",
                branch.id
            )));
        }
        if branch.expr.trim().is_empty() {
            return Err(InfrastructureAclError::Validation(format!(
                "rule_graph node '{node_id}' match branch '{}' has empty expr",
                branch.id
            )));
        }
        if branch.target_node_id.trim().is_empty()
            || !node_ids.contains(branch.target_node_id.as_str())
        {
            return Err(InfrastructureAclError::Validation(format!(
                "rule_graph node '{node_id}' match branch '{}' references missing target '{}'",
                branch.id, branch.target_node_id
            )));
        }
    }

    if let Some(fallback) = config.fallback_node_id.as_deref()
        && (fallback.trim().is_empty() || !node_ids.contains(fallback))
    {
        return Err(InfrastructureAclError::Validation(format!(
            "rule_graph node '{node_id}' references missing fallback target '{fallback}'"
        )));
    }

    Ok(())
}

fn validate_code_runner_node(
    node_id: &str,
    config: Option<&RawCodeRunnerNodeConfig>,
) -> Result<(), InfrastructureAclError> {
    let Some(config) = config else {
        return Err(InfrastructureAclError::Validation(format!(
            "rule_graph node '{node_id}' missing code_runner config"
        )));
    };
    if config.timeout_ms == 0 {
        return Err(InfrastructureAclError::Validation(format!(
            "rule_graph node '{node_id}' timeout_ms must be greater than zero"
        )));
    }
    if config.max_memory_bytes == 0 {
        return Err(InfrastructureAclError::Validation(format!(
            "rule_graph node '{node_id}' max_memory_bytes must be greater than zero"
        )));
    }
    if config.code.trim().is_empty() {
        return Err(InfrastructureAclError::Validation(format!(
            "rule_graph node '{node_id}' code cannot be empty"
        )));
    }
    Ok(())
}

fn validate_wasm_plugin_paths(
    node_id: &str,
    field: &str,
    paths: &[String],
) -> Result<(), InfrastructureAclError> {
    if paths.is_empty() {
        return Ok(());
    }

    for path in paths {
        if path.trim().is_empty() {
            return Err(InfrastructureAclError::Validation(format!(
                "rule_graph node '{node_id}' {field} cannot contain empty paths"
            )));
        }
        let path_ref = Path::new(path);
        if path_ref.is_absolute() {
            return Err(InfrastructureAclError::Validation(format!(
                "rule_graph node '{node_id}' {field} must use relative paths"
            )));
        }
        if path_ref.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::Prefix(_) | Component::RootDir
            )
        }) {
            return Err(InfrastructureAclError::Validation(format!(
                "rule_graph node '{node_id}' {field} must not contain parent traversal"
            )));
        }
    }

    Ok(())
}

fn validate_wasm_plugin_hosts(
    node_id: &str,
    hosts: &[String],
) -> Result<(), InfrastructureAclError> {
    if hosts.is_empty() {
        return Ok(());
    }

    for host in hosts {
        if host.trim().is_empty() {
            return Err(InfrastructureAclError::Validation(format!(
                "rule_graph node '{node_id}' allowed_hosts cannot contain empty hosts"
            )));
        }
    }

    Ok(())
}

fn validate_header_rule_target(
    rule: &RawHeaderRule,
    ids: &HashSet<&str>,
    kind: &str,
) -> Result<(), InfrastructureAclError> {
    let Some(target_id) = rule.target_id.as_deref() else {
        return Err(InfrastructureAclError::Validation(format!(
            "header_rule '{}' requires target_id for {kind} scope",
            rule.id
        )));
    };

    if !ids.contains(target_id) {
        return Err(InfrastructureAclError::Validation(format!(
            "header_rule '{}' references missing {kind} '{}'",
            rule.id, target_id
        )));
    }

    Ok(())
}

fn unique_ids<'a>(
    ids: impl IntoIterator<Item = &'a str>,
    kind: &str,
) -> Result<HashSet<&'a str>, InfrastructureAclError> {
    let mut seen = HashSet::new();
    for id in ids {
        if id.trim().is_empty() {
            return Err(InfrastructureAclError::Validation(format!(
                "{kind} id cannot be empty"
            )));
        }
        if !seen.insert(id) {
            return Err(InfrastructureAclError::Validation(format!(
                "duplicate {kind} id '{id}'"
            )));
        }
    }
    Ok(seen)
}

fn default_wasm_plugin_timeout_ms() -> u64 {
    20
}

pub fn map_gateway_catalog(
    raw_providers: &[RawProvider],
    raw_models: &[RawModel],
) -> Result<GatewayCatalog, InfrastructureAclError> {
    let providers = raw_providers
        .iter()
        .map(|provider| {
            Ok(Provider::new(
                ProviderId::new(provider.id.clone())?,
                provider.name.clone(),
            )?)
        })
        .collect::<Result<Vec<_>, InfrastructureAclError>>()?;
    let models = raw_models
        .iter()
        .map(|model| {
            Ok(Model::new(
                ModelId::new(model.id.clone())?,
                ProviderId::new(model.provider_id.clone())?,
                model.name.clone(),
            )?)
        })
        .collect::<Result<Vec<_>, InfrastructureAclError>>()?;

    let provider_catalog = ProviderCatalog::new(providers)?;
    let model_catalog = ModelCatalog::new(models)?;

    Ok(GatewayCatalog::new(provider_catalog, model_catalog)?)
}

pub fn map_workflow_index(
    raw_workflow_index: &RawWorkflowIndex,
) -> Result<WorkflowIndex, InfrastructureAclError> {
    let workflows = raw_workflow_index
        .workflows
        .iter()
        .map(|workflow| {
            let workflow_id = WorkflowId::new(workflow.id.clone())?;
            // Temporary bridge for the minimal raw workflow subset: we only have workflow IDs
            // in this slice, so we map each workflow to one route with the same identifier.
            let route_id = RouteId::new(workflow_id.as_str().to_string())?;
            Ok(Workflow::new(workflow_id, vec![route_id])?)
        })
        .collect::<Result<Vec<_>, InfrastructureAclError>>()?;
    let active_workflow_id = raw_workflow_index
        .active_workflow_id
        .as_ref()
        .map(WorkflowId::new)
        .transpose()?;

    Ok(WorkflowIndex::new(workflows, active_workflow_id)?)
}

fn validate_workflow_index_subset(raw: &RawGatewayConfig) -> Result<(), InfrastructureAclError> {
    if let Some(workflows_dir) = raw.workflows_dir.as_deref() {
        validate_relative_path("workflows_dir", workflows_dir)?;
    }

    let mut workflow_files = std::collections::HashSet::new();
    for workflow in &raw.workflows {
        let file = workflow.file.as_deref().ok_or_else(|| {
            InfrastructureAclError::Validation(format!(
                "workflow '{}' file cannot be empty",
                workflow.id
            ))
        })?;
        if file.trim().is_empty() {
            return Err(InfrastructureAclError::Validation(format!(
                "workflow '{}' file cannot be empty",
                workflow.id
            )));
        }
        if !workflow_files.insert(file.to_string()) {
            return Err(InfrastructureAclError::Validation(format!(
                "duplicate workflow file '{}'",
                file
            )));
        }
        validate_relative_path(&format!("workflow '{}' file", workflow.id), file)?;
    }

    if !raw.workflows.is_empty() {
        let active_id = raw.active_workflow_id.as_deref().ok_or_else(|| {
            InfrastructureAclError::Validation(
                "active_workflow_id must be set when workflows are present".to_string(),
            )
        })?;
        if !raw
            .workflows
            .iter()
            .any(|workflow| workflow.id == active_id)
        {
            return Err(InfrastructureAclError::Validation(format!(
                "active_workflow_id '{}' does not reference an indexed workflow",
                active_id
            )));
        }
    }

    Ok(())
}

fn validate_relative_path(field: &str, value: &str) -> Result<(), InfrastructureAclError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(InfrastructureAclError::Validation(format!(
            "{field} cannot be empty"
        )));
    }

    let path = Path::new(value);
    if path.is_absolute() {
        return Err(InfrastructureAclError::Validation(format!(
            "{field} must use relative paths"
        )));
    }
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::Prefix(_) | Component::RootDir
        )
    }) {
        return Err(InfrastructureAclError::Validation(format!(
            "{field} must not contain parent traversal"
        )));
    }

    Ok(())
}
