use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use application::{
    ensure_active_workflow_loaded, load_indexed_workflows, resolve_indexed_workflow_path,
    resolve_indexed_workflows_dir, validate_gateway_basics, validate_rule_code_runner_node,
    validate_rule_condition_node, validate_rule_copy_header_node, validate_rule_graph_structure,
    validate_rule_header_mutation_node, validate_rule_header_name_node, validate_rule_log_node,
    validate_rule_match_node, validate_rule_route_provider_node, validate_rule_router_node,
    validate_rule_select_model_node, validate_rule_set_context_node, validate_rule_value_node,
    validate_rule_wasm_plugin_node, ConditionModeInput, GatewayValidationInput,
    HeaderRuleValidationInput, MatchBranchInput, ModelValidationInput, ProviderValidationInput,
    RouteValidationInput, RouterClauseInput, RouterRuleInput, RuleGraphEdgeValidationInput,
    RuleGraphNodeValidationInput, RuleGraphStructureInput, RuleScopeInput, WasmCapabilityInput,
    WorkflowIndexEntryInput, WorkflowValidationInput,
};
use crate::atomic_store::write_toml_atomic;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    #[serde(default = "default_listen")]
    pub listen: String,
    #[serde(default = "default_admin_listen")]
    pub admin_listen: String,
    #[serde(default)]
    pub default_secret_env: Option<String>,
    #[serde(default)]
    pub providers: Vec<ProviderConfig>,
    #[serde(default)]
    pub models: Vec<ModelConfig>,
    #[serde(default)]
    pub routes: Vec<RouteConfig>,
    #[serde(default)]
    pub header_rules: Vec<HeaderRuleConfig>,
    #[serde(default)]
    pub rule_graph: Option<RuleGraphConfig>,
    #[serde(default)]
    pub workflows_dir: Option<String>,
    #[serde(default)]
    pub active_workflow_id: Option<String>,
    #[serde(default)]
    pub workflows: Vec<WorkflowIndexEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowIndexEntry {
    pub id: String,
    pub name: String,
    pub file: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowFileConfig {
    pub workflow: RuleGraphConfig,
}

#[derive(Debug, Clone, Default)]
pub struct LoadedWorkflowSet {
    pub summaries: Vec<WorkflowIndexEntry>,
    pub by_id: BTreeMap<String, WorkflowFileConfig>,
    pub active_workflow_id: Option<String>,
    legacy_rule_graph: Option<RuleGraphConfig>,
}

impl LoadedWorkflowSet {
    pub fn active_graph(&self) -> Option<&RuleGraphConfig> {
        self.active_workflow_id
            .as_deref()
            .and_then(|workflow_id| self.by_id.get(workflow_id))
            .map(|workflow| &workflow.workflow)
            .or(self.legacy_rule_graph.as_ref())
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeState {
    pub config: GatewayConfig,
    pub workflow_set: LoadedWorkflowSet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    pub base_url: String,
    #[serde(default)]
    pub default_headers: Vec<HeaderConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub id: String,
    pub name: String,
    pub provider_id: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteConfig {
    pub id: String,
    #[serde(default)]
    pub priority: i64,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(alias = "match")]
    pub matcher: String,
    pub provider_id: String,
    #[serde(default)]
    pub model_id: Option<String>,
    #[serde(default)]
    pub path_rewrite: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderRuleConfig {
    pub id: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    pub scope: RuleScope,
    #[serde(default)]
    pub target_id: Option<String>,
    #[serde(default)]
    pub when: Option<String>,
    #[serde(default)]
    pub actions: Vec<HeaderActionConfig>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuleScope {
    Global,
    Provider,
    Model,
    Route,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HeaderActionConfig {
    Set { name: String, value: String },
    Remove { name: String },
    Copy { from: String, to: String },
    SetIfAbsent { name: String, value: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderConfig {
    pub name: String,
    #[serde(flatten)]
    pub value: HeaderValueConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HeaderValueConfig {
    Encrypted {
        value: String,
        encrypted: bool,
        #[serde(default)]
        secret_env: Option<String>,
    },
    Plain {
        value: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleGraphConfig {
    #[serde(default = "default_rule_graph_version")]
    pub version: u32,
    pub start_node_id: String,
    #[serde(default)]
    pub nodes: Vec<RuleGraphNode>,
    #[serde(default)]
    pub edges: Vec<RuleGraphEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleGraphNode {
    pub id: String,
    #[serde(rename = "type")]
    pub node_type: RuleGraphNodeType,
    pub position: GraphPosition,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub condition: Option<ConditionNodeConfig>,
    #[serde(default)]
    pub route_provider: Option<RouteProviderNodeConfig>,
    #[serde(default)]
    pub select_model: Option<SelectModelNodeConfig>,
    #[serde(default)]
    pub rewrite_path: Option<ValueNodeConfig>,
    #[serde(default)]
    pub set_context: Option<SetContextNodeConfig>,
    #[serde(default)]
    pub router: Option<RouterNodeConfig>,
    #[serde(default)]
    pub log: Option<LogNodeConfig>,
    #[serde(default)]
    pub set_header: Option<HeaderMutationNodeConfig>,
    #[serde(default)]
    pub remove_header: Option<HeaderNameNodeConfig>,
    #[serde(default)]
    pub copy_header: Option<CopyHeaderNodeConfig>,
    #[serde(default)]
    pub set_header_if_absent: Option<HeaderMutationNodeConfig>,
    #[serde(default)]
    pub note_node: Option<NoteNodeConfig>,
    #[serde(default)]
    pub wasm_plugin: Option<WasmPluginNodeConfig>,
    #[serde(default, rename = "match", alias = "wasm_match")]
    pub match_node: Option<MatchNodeConfig>,
    #[serde(default)]
    pub code_runner: Option<CodeRunnerNodeConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphPosition {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleGraphEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub source_handle: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuleGraphNodeType {
    Start,
    Condition,
    RouteProvider,
    SelectModel,
    RewritePath,
    SetContext,
    Router,
    Log,
    SetHeader,
    RemoveHeader,
    CopyHeader,
    SetHeaderIfAbsent,
    WasmPlugin,
    #[serde(alias = "match")]
    Match,
    CodeRunner,
    Note,
    End,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionNodeConfig {
    pub mode: ConditionMode,
    #[serde(default)]
    pub expression: Option<String>,
    #[serde(default)]
    pub builder: Option<ConditionBuilderConfig>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConditionMode {
    Builder,
    Expression,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionBuilderConfig {
    pub field: String,
    pub operator: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteProviderNodeConfig {
    pub provider_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectModelNodeConfig {
    #[serde(default)]
    pub provider_id: String,
    pub model_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueNodeConfig {
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetContextNodeConfig {
    pub key: String,
    pub value_template: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterNodeConfig {
    #[serde(default)]
    pub rules: Vec<RouterRuleConfig>,
    #[serde(default)]
    pub fallback_node_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterRuleConfig {
    pub id: String,
    #[serde(default)]
    pub clauses: Vec<RouterClauseConfig>,
    pub target_node_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterClauseConfig {
    pub source: String,
    pub operator: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogNodeConfig {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderMutationNodeConfig {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderNameNodeConfig {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyHeaderNodeConfig {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteNodeConfig {
    #[serde(default)]
    pub text: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum WasmCapability {
    Log,
    Fs,
    Network,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmPluginNodeConfig {
    pub plugin_id: String,
    #[serde(default = "default_wasm_plugin_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default)]
    pub fuel: Option<u64>,
    pub max_memory_bytes: u64,
    #[serde(default)]
    pub granted_capabilities: Vec<WasmCapability>,
    #[serde(default)]
    pub read_dirs: Vec<String>,
    #[serde(default)]
    pub write_dirs: Vec<String>,
    #[serde(default)]
    pub allowed_hosts: Vec<String>,
    #[serde(default)]
    pub config: toml::value::Table,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchNodeConfig {
    #[serde(flatten)]
    pub plugin: WasmPluginNodeConfig,
    #[serde(default)]
    pub branches: Vec<MatchBranchConfig>,
    #[serde(default)]
    pub fallback_node_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchBranchConfig {
    pub id: String,
    pub expr: String,
    pub target_node_id: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CodeRunnerLanguage {
    Javascript,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeRunnerNodeConfig {
    pub language: CodeRunnerLanguage,
    #[serde(default = "default_wasm_plugin_timeout_ms")]
    pub timeout_ms: u64,
    pub max_memory_bytes: u64,
    pub code: String,
}

pub fn load_config(path: &Path) -> Result<GatewayConfig, Box<dyn std::error::Error>> {
    let raw = std::fs::read_to_string(path)?;
    let config = parse_config(&raw)?;
    Ok(config)
}

pub fn load_runtime_state(path: &Path) -> Result<RuntimeState, Box<dyn std::error::Error>> {
    let config = load_config(path)?;
    runtime_state_from_config(path, config)
}

pub fn resolve_workflows_dir(config_path: &Path, config: &GatewayConfig) -> Option<PathBuf> {
    resolve_indexed_workflows_dir(config_path, config.workflows_dir.as_deref())
}

pub fn resolve_workflow_path(
    config_path: &Path,
    config: &GatewayConfig,
    workflow_file: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    resolve_indexed_workflow_path(config_path, config.workflows_dir.as_deref(), workflow_file)
        .map_err(|error| error.to_string().into())
}

pub fn load_workflow_file(path: &Path) -> Result<WorkflowFileConfig, Box<dyn std::error::Error>> {
    let raw = std::fs::read_to_string(path)?;
    Ok(toml::from_str(&raw)?)
}

pub fn save_workflow_file_atomic(
    path: &Path,
    workflow: &WorkflowFileConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    write_toml_atomic(path, workflow)?;
    Ok(())
}

pub fn load_workflow_set(
    config_path: &Path,
    config: &GatewayConfig,
) -> Result<LoadedWorkflowSet, Box<dyn std::error::Error>> {
    let provider_ids = unique_ids(
        config.providers.iter().map(|provider| provider.id.as_str()),
        "provider",
    )?;
    let model_ids = unique_ids(config.models.iter().map(|model| model.id.as_str()), "model")?;
    let workflow_entries = config
        .workflows
        .iter()
        .map(|workflow| WorkflowIndexEntryInput {
            id: workflow.id.clone(),
            file: workflow.file.clone(),
        })
        .collect::<Vec<_>>();
    let allow_legacy_missing_file_fallback = uses_synthesized_legacy_workflow_index(config);
    let by_id = load_indexed_workflows(
        config_path,
        config.workflows_dir.as_deref(),
        &workflow_entries,
        allow_legacy_missing_file_fallback,
        load_workflow_file,
        |error| is_not_found_error(error.as_ref()),
        |workflow_id, workflow_path, loaded| {
            validate_rule_graph(&loaded.workflow, &provider_ids, &model_ids, &config.models)
                .map_err(|error| {
                    format!(
                        "workflow '{}' in '{}' is invalid: {error}",
                        workflow_id,
                        workflow_path.display()
                    )
                })
        },
    )
    .map_err(|error| error.to_string())?;

    ensure_active_workflow_loaded(
        config.active_workflow_id.as_deref(),
        !config.workflows.is_empty(),
        &by_id,
        allow_legacy_missing_file_fallback,
    )
    .map_err(|error| error.to_string())?;

    Ok(LoadedWorkflowSet {
        summaries: config.workflows.clone(),
        by_id,
        active_workflow_id: config.active_workflow_id.clone(),
        legacy_rule_graph: config.rule_graph.clone(),
    })
}

pub fn runtime_state_from_config(
    config_path: &Path,
    config: GatewayConfig,
) -> Result<RuntimeState, Box<dyn std::error::Error>> {
    let workflow_set = load_workflow_set(config_path, &config)?;
    Ok(RuntimeState {
        config,
        workflow_set,
    })
}

pub fn save_config_atomic(
    path: &Path,
    config: &GatewayConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let normalized = normalize_legacy_rule_graph(config.clone());
    validate_config(&normalized)?;
    write_toml_atomic(path, &normalized)?;
    Ok(())
}

pub fn parse_config(raw: &str) -> Result<GatewayConfig, Box<dyn std::error::Error>> {
    let config: GatewayConfig = toml::from_str(raw)?;
    let config = normalize_legacy_rule_graph(config);
    validate_config(&config)?;
    Ok(config)
}

pub fn validate_config(config: &GatewayConfig) -> Result<(), Box<dyn std::error::Error>> {
    let basic_validation = validate_gateway_basics(&GatewayValidationInput {
        workflows_dir: config.workflows_dir.clone(),
        active_workflow_id: config.active_workflow_id.clone(),
        providers: config
            .providers
            .iter()
            .map(|provider| ProviderValidationInput {
                id: provider.id.clone(),
                base_url: provider.base_url.clone(),
            })
            .collect(),
        models: config
            .models
            .iter()
            .map(|model| ModelValidationInput {
                id: model.id.clone(),
                provider_id: model.provider_id.clone(),
            })
            .collect(),
        routes: config
            .routes
            .iter()
            .map(|route| RouteValidationInput {
                id: route.id.clone(),
                matcher: route.matcher.clone(),
                provider_id: route.provider_id.clone(),
                model_id: route.model_id.clone(),
            })
            .collect(),
        header_rules: config
            .header_rules
            .iter()
            .map(|rule| HeaderRuleValidationInput {
                id: rule.id.clone(),
                scope: map_rule_scope(rule.scope),
                target_id: rule.target_id.clone(),
                actions_len: rule.actions.len(),
            })
            .collect(),
        workflows: config
            .workflows
            .iter()
            .map(|workflow| WorkflowValidationInput {
                id: workflow.id.clone(),
                file: workflow.file.clone(),
            })
            .collect(),
    })
    .map_err(|error| error.to_string())?;
    let provider_ids = basic_validation
        .provider_ids
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let model_ids = basic_validation
        .model_ids
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();

    if let Some(graph) = &config.rule_graph {
        validate_rule_graph(graph, &provider_ids, &model_ids, &config.models)?;
    }

    Ok(())
}

fn map_rule_scope(scope: RuleScope) -> RuleScopeInput {
    match scope {
        RuleScope::Global => RuleScopeInput::Global,
        RuleScope::Provider => RuleScopeInput::Provider,
        RuleScope::Model => RuleScopeInput::Model,
        RuleScope::Route => RuleScopeInput::Route,
    }
}

pub fn normalize_legacy_rule_graph(mut config: GatewayConfig) -> GatewayConfig {
    let Some(graph) = config.rule_graph.take() else {
        return normalize_workflow_index_inputs(config);
    };

    let node_map = graph
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect::<HashMap<_, _>>();

    let incoming_edges = graph.edges.iter().fold(
        HashMap::<&str, Vec<&RuleGraphEdge>>::new(),
        |mut acc, edge| {
            acc.entry(edge.target.as_str()).or_default().push(edge);
            acc
        },
    );
    let outgoing_edges = graph.edges.iter().fold(
        HashMap::<&str, Vec<&RuleGraphEdge>>::new(),
        |mut acc, edge| {
            acc.entry(edge.source.as_str()).or_default().push(edge);
            acc
        },
    );

    let mut route_nodes_to_remove = HashSet::<String>::new();
    let mut updated_nodes = Vec::with_capacity(graph.nodes.len());
    let mut rewritten_edges = graph.edges.clone();

    for node in &graph.nodes {
        if node.node_type != RuleGraphNodeType::SelectModel {
            continue;
        }

        let Some(select_model) = &node.select_model else {
            continue;
        };
        if !select_model.provider_id.trim().is_empty() {
            continue;
        }

        let Some(incoming) = incoming_edges.get(node.id.as_str()) else {
            continue;
        };
        if incoming.len() != 1 {
            continue;
        }

        let route_edge = incoming[0];
        let Some(route_node) = node_map.get(route_edge.source.as_str()) else {
            continue;
        };
        if route_node.node_type != RuleGraphNodeType::RouteProvider {
            continue;
        }
        let Some(route_config) = route_node.route_provider.as_ref() else {
            continue;
        };

        let Some(route_outgoing) = outgoing_edges.get(route_node.id.as_str()) else {
            continue;
        };
        if route_outgoing.len() != 1 || route_outgoing[0].target != node.id {
            continue;
        }

        route_nodes_to_remove.insert(route_node.id.clone());
        updated_nodes.push(RuleGraphNode {
            select_model: Some(SelectModelNodeConfig {
                provider_id: route_config.provider_id.clone(),
                model_id: select_model.model_id.clone(),
            }),
            ..node.clone()
        });

        rewritten_edges = rewritten_edges
            .into_iter()
            .filter(|edge| edge.id != route_edge.id)
            .map(|edge| {
                if edge.target == route_node.id {
                    RuleGraphEdge {
                        target: node.id.clone(),
                        ..edge
                    }
                } else if edge.source == route_node.id {
                    RuleGraphEdge {
                        source: node.id.clone(),
                        ..edge
                    }
                } else {
                    edge
                }
            })
            .collect();
    }

    if route_nodes_to_remove.is_empty() {
        config.rule_graph = Some(graph);
        return normalize_workflow_index_inputs(synthesize_legacy_workflow_index(config));
    }

    let updated_node_ids = updated_nodes
        .iter()
        .map(|node| (node.id.clone(), node.clone()))
        .collect::<HashMap<_, _>>();

    config.rule_graph = Some(RuleGraphConfig {
        nodes: graph
            .nodes
            .into_iter()
            .filter(|node| !route_nodes_to_remove.contains(&node.id))
            .map(|node| updated_node_ids.get(&node.id).cloned().unwrap_or(node))
            .collect(),
        edges: rewritten_edges
            .into_iter()
            .filter(|edge| {
                !route_nodes_to_remove.contains(&edge.source)
                    && !route_nodes_to_remove.contains(&edge.target)
            })
            .collect(),
        ..graph
    });
    normalize_workflow_index_inputs(synthesize_legacy_workflow_index(config))
}

fn synthesize_legacy_workflow_index(mut config: GatewayConfig) -> GatewayConfig {
    if config.rule_graph.is_some() && config.workflows.is_empty() {
        let synthesized_workflow_id = config
            .active_workflow_id
            .clone()
            .unwrap_or_else(|| "default".to_string());

        config
            .workflows_dir
            .get_or_insert_with(|| "workflows".to_string());
        config
            .active_workflow_id
            .get_or_insert_with(|| synthesized_workflow_id.clone());
        config.workflows = vec![WorkflowIndexEntry {
            id: synthesized_workflow_id.clone(),
            name: if synthesized_workflow_id == "default" {
                "Default Workflow".to_string()
            } else {
                format!("Workflow {synthesized_workflow_id}")
            },
            file: format!("{synthesized_workflow_id}.toml"),
            description: Some("Migrated from legacy rule_graph".to_string()),
        }];
    }

    config
}

fn normalize_workflow_index_inputs(mut config: GatewayConfig) -> GatewayConfig {
    if let Some(workflows_dir) = config.workflows_dir.as_mut() {
        *workflows_dir = workflows_dir.trim().to_string();
    }
    for workflow in &mut config.workflows {
        workflow.file = workflow.file.trim().to_string();
    }

    config
}

fn uses_synthesized_legacy_workflow_index(config: &GatewayConfig) -> bool {
    if config.rule_graph.is_none() || config.workflows.len() != 1 {
        return false;
    }

    let Some(workflows_dir) = config.workflows_dir.as_deref() else {
        return false;
    };
    let Some(active_workflow_id) = config.active_workflow_id.as_deref() else {
        return false;
    };
    let workflow = &config.workflows[0];
    let expected_name = if active_workflow_id == "default" {
        "Default Workflow".to_string()
    } else {
        format!("Workflow {active_workflow_id}")
    };

    workflows_dir == "workflows"
        && workflow.id == active_workflow_id
        && workflow.name == expected_name
        && workflow.file == format!("{active_workflow_id}.toml")
        && workflow.description.as_deref() == Some("Migrated from legacy rule_graph")
}

fn is_not_found_error(error: &(dyn std::error::Error + 'static)) -> bool {
    error
        .downcast_ref::<std::io::Error>()
        .is_some_and(|io_error| io_error.kind() == std::io::ErrorKind::NotFound)
}

fn validate_rule_graph(
    graph: &RuleGraphConfig,
    provider_ids: &HashSet<&str>,
    model_ids: &HashSet<&str>,
    models: &[ModelConfig],
) -> Result<(), Box<dyn std::error::Error>> {
    let provider_ids_owned = provider_ids
        .iter()
        .map(|id| id.to_string())
        .collect::<HashSet<_>>();
    let model_ids_owned = model_ids
        .iter()
        .map(|id| id.to_string())
        .collect::<HashSet<_>>();
    let _ = validate_rule_graph_structure(&RuleGraphStructureInput {
        start_node_id: graph.start_node_id.clone(),
        nodes: graph
            .nodes
            .iter()
            .map(|node| RuleGraphNodeValidationInput {
                id: node.id.clone(),
                is_start: node.node_type == RuleGraphNodeType::Start,
            })
            .collect(),
        edges: graph
            .edges
            .iter()
            .map(|edge| RuleGraphEdgeValidationInput {
                id: edge.id.clone(),
                source: edge.source.clone(),
                target: edge.target.clone(),
            })
            .collect(),
    })
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
        RuleGraphNodeType::Condition => {
            let Some(condition) = &node.condition else {
                return Err(
                    format!("rule_graph node '{}' missing condition config", node.id).into(),
                );
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
        }
        RuleGraphNodeType::RouteProvider => {
            validate_rule_route_provider_node(
                node.id.as_str(),
                node.route_provider
                    .as_ref()
                    .map(|config| config.provider_id.as_str()),
                provider_ids_owned,
            )
            .map_err(|error| error.to_string())?;
        }
        RuleGraphNodeType::SelectModel => {
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
    let rules = config.map(|config| {
        config
            .rules
            .iter()
            .map(|rule| RouterRuleInput {
                id: rule.id.clone(),
                clauses: rule
                    .clauses
                    .iter()
                    .map(|clause| RouterClauseInput {
                        source: clause.source.clone(),
                        operator: clause.operator.clone(),
                        value: clause.value.clone(),
                    })
                    .collect(),
                target_node_id: rule.target_node_id.clone(),
            })
            .collect::<Vec<_>>()
    });
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
    validate_rule_wasm_plugin_node(
        node_id,
        Some(config.plugin_id.as_str()),
        Some(config.timeout_ms),
        Some(config.fuel),
        Some(config.max_memory_bytes),
        &config
            .granted_capabilities
            .iter()
            .copied()
            .map(map_wasm_capability)
            .collect::<Vec<_>>(),
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
    let branches = config
        .branches
        .iter()
        .map(|branch| MatchBranchInput {
            id: branch.id.clone(),
            expr: branch.expr.clone(),
            target_node_id: branch.target_node_id.clone(),
        })
        .collect::<Vec<_>>();
    validate_rule_match_node(
        node_id,
        &node_ids,
        Some(&branches),
        config.fallback_node_id.as_deref(),
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

fn map_condition_mode(mode: ConditionMode) -> ConditionModeInput {
    match mode {
        ConditionMode::Builder => ConditionModeInput::Builder,
        ConditionMode::Expression => ConditionModeInput::Expression,
    }
}

fn map_wasm_capability(capability: WasmCapability) -> WasmCapabilityInput {
    match capability {
        WasmCapability::Log => WasmCapabilityInput::Log,
        WasmCapability::Fs => WasmCapabilityInput::Fs,
        WasmCapability::Network => WasmCapabilityInput::Network,
    }
}

fn unique_ids<'a>(
    ids: impl IntoIterator<Item = &'a str>,
    kind: &str,
) -> Result<HashSet<&'a str>, Box<dyn std::error::Error>> {
    let mut seen = HashSet::new();
    for id in ids {
        if id.trim().is_empty() {
            return Err(format!("{kind} id cannot be empty").into());
        }
        if !seen.insert(id) {
            return Err(format!("duplicate {kind} id '{id}'").into());
        }
    }
    Ok(seen)
}

fn default_listen() -> String {
    "127.0.0.1:9001".to_string()
}

fn default_admin_listen() -> String {
    "127.0.0.1:9002".to_string()
}

fn default_rule_graph_version() -> u32 {
    1
}

fn default_wasm_plugin_timeout_ms() -> u64 {
    20
}

fn default_enabled() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::{
        load_config, load_workflow_file, load_workflow_set, normalize_legacy_rule_graph,
        parse_config, resolve_workflows_dir, CodeRunnerLanguage, GatewayConfig, GraphPosition,
        RuleGraphConfig, RuleGraphNode, RuleGraphNodeType, RuleScope, WasmCapability,
    };
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    const VALID_CONFIG: &str = r#"
listen = "127.0.0.1:9001"
admin_listen = "127.0.0.1:9002"

[[providers]]
id = "kimi"
name = "Kimi"
base_url = "https://api.kimi.com"

[[providers.default_headers]]
name = "Authorization"
value = "enc:v1:test"
encrypted = true

[[models]]
id = "kimi-k2"
name = "Kimi K2"
provider_id = "kimi"

[[routes]]
id = "chat-default"
priority = 100
enabled = true
matcher = 'path.startsWith("/v1/chat/completions") && method == "POST"'
provider_id = "kimi"
model_id = "kimi-k2"
path_rewrite = "/coding/v1/chat/completions"

[[header_rules]]
id = "inject-model-header"
enabled = true
scope = "model"
target_id = "kimi-k2"
when = 'path.startsWith("/v1/")'

[[header_rules.actions]]
type = "set"
name = "X-Model"
value = "${model.id}"

[rule_graph]
version = 1
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"
position = { x = 0.0, y = 0.0 }

[[rule_graph.nodes]]
id = "cond-1"
type = "condition"
position = { x = 120.0, y = 0.0 }

[rule_graph.nodes.condition]
mode = "expression"
expression = 'path.startsWith("/v1/")'

[[rule_graph.nodes]]
id = "provider-kimi"
type = "route_provider"
position = { x = 240.0, y = 0.0 }

[rule_graph.nodes.route_provider]
provider_id = "kimi"

[[rule_graph.nodes]]
id = "end"
type = "end"
position = { x = 360.0, y = 0.0 }

[[rule_graph.edges]]
id = "edge-1"
source = "start"
target = "cond-1"

[[rule_graph.edges]]
id = "edge-2"
source = "cond-1"
source_handle = "true"
target = "provider-kimi"

[[rule_graph.edges]]
id = "edge-3"
source = "provider-kimi"
target = "end"
"#;

    const VALID_WASM_PLUGIN_CONFIG: &str = r#"
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
position = { x = 0.0, y = 0.0 }

[[rule_graph.nodes]]
id = "plugin"
type = "wasm_plugin"
position = { x = 120.0, y = 0.0 }

[rule_graph.nodes.wasm_plugin]
plugin_id = "intent-classifier"
fuel = 500000
max_memory_bytes = 16777216
granted_capabilities = ["fs", "network"]
read_dirs = ["plugins-data/common"]
write_dirs = ["plugins-data/runtime"]
allowed_hosts = ["api.example.com:443"]

[rule_graph.nodes.wasm_plugin.config]
prompt = "classify request intent"
default_intent = "chat"

[[rule_graph.nodes]]
id = "end"
type = "end"
position = { x = 240.0, y = 0.0 }

[[rule_graph.edges]]
id = "edge-1"
source = "start"
target = "plugin"

[[rule_graph.edges]]
id = "edge-2"
source = "plugin"
target = "end"
"#;

    const VALID_WASM_MATCH_CONFIG: &str = r#"
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
position = { x = 0.0, y = 0.0 }

[[rule_graph.nodes]]
id = "matcher"
type = "match"
position = { x = 120.0, y = 0.0 }

[rule_graph.nodes.match]
plugin_id = "remote-policy-router"
fuel = 500000
max_memory_bytes = 16777216
granted_capabilities = ["log"]
fallback_node_id = "end"

[[rule_graph.nodes.match.branches]]
id = "chat"
expr = "ctx.header.x-tenant == enterprise"
target_node_id = "end"

[rule_graph.nodes.match.config]
match_header = "x-tenant"
fallback_port = "default"

[[rule_graph.nodes]]
id = "end"
type = "end"
position = { x = 240.0, y = 0.0 }

[[rule_graph.edges]]
id = "edge-1"
source = "start"
target = "matcher"

[[rule_graph.edges]]
id = "edge-2"
source = "matcher"
source_handle = "default"
target = "end"
"#;

    const VALID_CODE_RUNNER_CONFIG: &str = r#"
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
position = { x = 0.0, y = 0.0 }

[[rule_graph.nodes]]
id = "runner"
type = "code_runner"
position = { x = 120.0, y = 0.0 }

[rule_graph.nodes.code_runner]
language = "javascript"
timeout_ms = 25
max_memory_bytes = 16777216
code = "export function run(input) { return {}; }"

[[rule_graph.nodes]]
id = "end"
type = "end"
position = { x = 240.0, y = 0.0 }

[[rule_graph.edges]]
id = "edge-1"
source = "start"
target = "runner"

[[rule_graph.edges]]
id = "edge-2"
source = "runner"
target = "end"
"#;

    fn temp_dir(name: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be monotonic enough for tests")
            .as_nanos();
        dir.push(format!(
            "proxy-tools-config-{name}-{}-{stamp}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("temp dir should be creatable");
        dir
    }

    #[test]
    fn parses_structured_gateway_config() {
        let config = parse_config(VALID_CONFIG).expect("valid config should parse");

        assert_eq!(config.providers.len(), 1);
        assert_eq!(config.models.len(), 1);
        assert_eq!(config.routes.len(), 1);
        assert_eq!(config.header_rules.len(), 1);
        assert_eq!(config.header_rules[0].scope, RuleScope::Model);
        assert!(config.rule_graph.is_some());
    }

    #[test]
    fn parses_workflow_index_metadata() {
        let config = parse_config(
            r#"
listen = "127.0.0.1:9001"
admin_listen = "127.0.0.1:9002"
workflows_dir = "workflows"
active_workflow_id = "chat-routing"

[[workflows]]
id = "chat-routing"
name = "Chat Routing"
file = "chat-routing.toml"
description = "Main chat flow"
"#,
        )
        .expect("config should parse");

        assert_eq!(config.workflows_dir.as_deref(), Some("workflows"));
        assert_eq!(config.active_workflow_id.as_deref(), Some("chat-routing"));
        assert_eq!(config.workflows.len(), 1);
        assert_eq!(config.workflows[0].file, "chat-routing.toml");
    }

    #[test]
    fn loads_workflow_file_from_workflows_dir() {
        let root = temp_dir("load-workflow-file");
        let config_path = root.join("gateway.toml");
        let workflows_dir = root.join("workflows");

        fs::create_dir_all(&workflows_dir).expect("workflows dir should be creatable");
        fs::write(
            &config_path,
            r#"
listen = "127.0.0.1:9001"
admin_listen = "127.0.0.1:9002"
workflows_dir = "workflows"
active_workflow_id = "chat-routing"

[[workflows]]
id = "chat-routing"
name = "Chat Routing"
file = "chat-routing.toml"
"#,
        )
        .expect("config should be writable");
        fs::write(
            workflows_dir.join("chat-routing.toml"),
            r#"
[workflow]
version = 1
start_node_id = "start"

[[workflow.nodes]]
id = "start"
type = "start"
position = { x = 0.0, y = 0.0 }
"#,
        )
        .expect("workflow should be writable");

        let loaded = load_config(&config_path).expect("config should load");
        let resolved_dir =
            resolve_workflows_dir(&config_path, &loaded).expect("workflows dir should resolve");
        let workflow = load_workflow_file(&resolved_dir.join("chat-routing.toml"))
            .expect("workflow file should load");

        assert_eq!(loaded.active_workflow_id.as_deref(), Some("chat-routing"));
        assert_eq!(workflow.workflow.start_node_id, "start");
        assert_eq!(workflow.workflow.nodes.len(), 1);
    }

    #[test]
    fn rejects_missing_indexed_workflow_file_even_with_legacy_rule_graph() {
        let root = temp_dir("missing-indexed-workflow");
        let config_path = root.join("gateway.toml");

        let config = parse_config(
            r#"
listen = "127.0.0.1:9001"
admin_listen = "127.0.0.1:9002"
workflows_dir = "workflows"
active_workflow_id = "chat-routing"

[[workflows]]
id = "chat-routing"
name = "Chat Routing"
file = "chat-routing.toml"

[rule_graph]
version = 1
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"
position = { x = 0.0, y = 0.0 }
"#,
        )
        .expect("config should parse");

        let error = load_workflow_set(&config_path, &config)
            .expect_err("missing indexed workflow should fail");
        assert!(
            error
                .to_string()
                .contains("failed to load workflow 'chat-routing'"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn allows_missing_synthesized_legacy_workflow_file() {
        let root = temp_dir("legacy-missing-workflow");
        let config_path = root.join("gateway.toml");

        let config = parse_config(
            r#"
listen = "127.0.0.1:9001"
admin_listen = "127.0.0.1:9002"

[rule_graph]
version = 1
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"
position = { x = 0.0, y = 0.0 }
"#,
        )
        .expect("legacy config should parse");

        let workflow_set =
            load_workflow_set(&config_path, &config).expect("legacy fallback should load");
        let active = workflow_set
            .active_graph()
            .expect("legacy rule graph should still be active");

        assert_eq!(workflow_set.by_id.len(), 0);
        assert_eq!(active.start_node_id, "start");
    }

    #[test]
    fn trims_workflow_index_path_inputs_during_parse() {
        let config = parse_config(
            r#"
listen = "127.0.0.1:9001"
admin_listen = "127.0.0.1:9002"
workflows_dir = " workflows "
active_workflow_id = "default"

[[workflows]]
id = "default"
name = "Default"
file = " default.toml "
"#,
        )
        .expect("config should parse");

        assert_eq!(config.workflows_dir.as_deref(), Some("workflows"));
        assert_eq!(config.workflows[0].file, "default.toml");
    }

    #[test]
    fn normalizes_workflow_index_inputs_for_legacy_rule_graph_configs() {
        let normalized = parse_config(
            r#"
listen = "127.0.0.1:9001"
admin_listen = "127.0.0.1:9002"
workflows_dir = " workflows "
active_workflow_id = "default"

[[workflows]]
id = "default"
name = "Default"
file = " default.toml "

[rule_graph]
version = 1
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"
position = { x = 0.0, y = 0.0 }
"#,
        )
        .expect("legacy config should normalize");

        assert_eq!(normalized.workflows_dir.as_deref(), Some("workflows"));
        assert_eq!(normalized.workflows[0].file, "default.toml");

        let duplicate = parse_config(
            r#"
listen = "127.0.0.1:9001"
admin_listen = "127.0.0.1:9002"
workflows_dir = " workflows "
active_workflow_id = "default"

[[workflows]]
id = "default"
name = "Default"
file = " default.toml "

[[workflows]]
id = "secondary"
name = "Secondary"
file = " default.toml "

[rule_graph]
version = 1
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"
position = { x = 0.0, y = 0.0 }
"#,
        )
        .expect_err("duplicate workflow files should fail after trimming");

        assert!(
            duplicate
                .to_string()
                .contains("duplicate workflow file 'default.toml'"),
            "unexpected error: {duplicate}"
        );
    }

    #[test]
    fn normalizes_legacy_rule_graph_into_default_workflow_index() {
        let legacy = GatewayConfig {
            listen: "127.0.0.1:9001".to_string(),
            admin_listen: "127.0.0.1:9002".to_string(),
            default_secret_env: None,
            providers: Vec::new(),
            models: Vec::new(),
            routes: Vec::new(),
            header_rules: Vec::new(),
            rule_graph: Some(RuleGraphConfig {
                version: 1,
                start_node_id: "start".to_string(),
                nodes: vec![RuleGraphNode {
                    id: "start".to_string(),
                    node_type: RuleGraphNodeType::Start,
                    position: GraphPosition { x: 0.0, y: 0.0 },
                    note: None,
                    condition: None,
                    route_provider: None,
                    select_model: None,
                    rewrite_path: None,
                    set_context: None,
                    router: None,
                    log: None,
                    set_header: None,
                    remove_header: None,
                    copy_header: None,
                    set_header_if_absent: None,
                    note_node: None,
                    wasm_plugin: None,
                    match_node: None,
                    code_runner: None,
                }],
                edges: Vec::new(),
            }),
            workflows_dir: None,
            active_workflow_id: None,
            workflows: Vec::new(),
        };

        let normalized = normalize_legacy_rule_graph(legacy);
        assert_eq!(normalized.active_workflow_id.as_deref(), Some("default"));
        assert_eq!(normalized.workflows.len(), 1);
        assert!(normalized.rule_graph.is_some());
        assert_eq!(normalized.workflows_dir.as_deref(), Some("workflows"));
        let graph = normalized
            .rule_graph
            .expect("graph should remain available");
        assert_eq!(graph.start_node_id, "start");
    }

    #[test]
    fn preserves_user_supplied_workflow_metadata_during_legacy_synthesis() {
        let legacy = GatewayConfig {
            listen: "127.0.0.1:9001".to_string(),
            admin_listen: "127.0.0.1:9002".to_string(),
            default_secret_env: None,
            providers: Vec::new(),
            models: Vec::new(),
            routes: Vec::new(),
            header_rules: Vec::new(),
            rule_graph: Some(RuleGraphConfig {
                version: 1,
                start_node_id: "start".to_string(),
                nodes: vec![RuleGraphNode {
                    id: "start".to_string(),
                    node_type: RuleGraphNodeType::Start,
                    position: GraphPosition { x: 0.0, y: 0.0 },
                    note: None,
                    condition: None,
                    route_provider: None,
                    select_model: None,
                    rewrite_path: None,
                    set_context: None,
                    router: None,
                    log: None,
                    set_header: None,
                    remove_header: None,
                    copy_header: None,
                    set_header_if_absent: None,
                    note_node: None,
                    wasm_plugin: None,
                    match_node: None,
                    code_runner: None,
                }],
                edges: Vec::new(),
            }),
            workflows_dir: Some("custom-workflows".to_string()),
            active_workflow_id: Some("custom-active".to_string()),
            workflows: Vec::new(),
        };

        let normalized = normalize_legacy_rule_graph(legacy);
        assert_eq!(
            normalized.workflows_dir.as_deref(),
            Some("custom-workflows")
        );
        assert_eq!(
            normalized.active_workflow_id.as_deref(),
            Some("custom-active")
        );
        assert_eq!(normalized.workflows.len(), 1);
        assert!(normalized.rule_graph.is_some());
    }

    #[test]
    fn parses_legacy_rule_graph_with_custom_active_workflow_id() {
        let config = parse_config(
            r#"
listen = "127.0.0.1:9001"
admin_listen = "127.0.0.1:9002"
active_workflow_id = "chat-routing"

[rule_graph]
version = 1
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"
position = { x = 0.0, y = 0.0 }
"#,
        )
        .expect("legacy config should normalize");

        assert_eq!(config.active_workflow_id.as_deref(), Some("chat-routing"));
        assert_eq!(config.workflows_dir.as_deref(), Some("workflows"));
        assert_eq!(config.workflows.len(), 1);
        assert_eq!(config.workflows[0].id, "chat-routing");
        assert_eq!(config.workflows[0].file, "chat-routing.toml");
        assert!(config.rule_graph.is_some());
    }

    #[test]
    fn rejects_duplicate_workflow_ids() {
        let invalid = r#"
listen = "127.0.0.1:9001"
admin_listen = "127.0.0.1:9002"
active_workflow_id = "default"

[[workflows]]
id = "default"
name = "Default"
file = "default.toml"

[[workflows]]
id = "default"
name = "Duplicate"
file = "duplicate.toml"
"#;

        let error = parse_config(invalid).expect_err("duplicate workflow ids should fail");
        assert!(
            error
                .to_string()
                .contains("duplicate workflow id 'default'"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn rejects_empty_workflow_file() {
        let invalid = r#"
listen = "127.0.0.1:9001"
admin_listen = "127.0.0.1:9002"
active_workflow_id = "default"

[[workflows]]
id = "default"
name = "Default"
file = ""
"#;

        let error = parse_config(invalid).expect_err("empty workflow file should fail");
        assert!(
            error
                .to_string()
                .contains("workflow 'default' file cannot be empty"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn rejects_absolute_workflow_file_path() {
        let invalid = r#"
listen = "127.0.0.1:9001"
admin_listen = "127.0.0.1:9002"
active_workflow_id = "default"

[[workflows]]
id = "default"
name = "Default"
file = "/tmp/default.toml"
"#;

        let error = parse_config(invalid).expect_err("absolute workflow file should fail");
        assert!(
            error
                .to_string()
                .contains("workflow 'default' file must use relative paths"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn rejects_parent_traversal_in_workflow_file() {
        let invalid = r#"
listen = "127.0.0.1:9001"
admin_listen = "127.0.0.1:9002"
active_workflow_id = "default"

[[workflows]]
id = "default"
name = "Default"
file = "../default.toml"
"#;

        let error = parse_config(invalid).expect_err("parent traversal should fail");
        assert!(
            error
                .to_string()
                .contains("workflow 'default' file must not contain parent traversal"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn rejects_invalid_workflows_dir() {
        let invalid = r#"
listen = "127.0.0.1:9001"
admin_listen = "127.0.0.1:9002"
workflows_dir = " /tmp/workflows "
active_workflow_id = "default"

[[workflows]]
id = "default"
name = "Default"
file = "default.toml"
"#;

        let error = parse_config(invalid).expect_err("invalid workflows_dir should fail");
        assert!(
            error
                .to_string()
                .contains("workflows_dir must use relative paths"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn rejects_whitespace_workflows_dir() {
        let invalid = r#"
listen = "127.0.0.1:9001"
admin_listen = "127.0.0.1:9002"
workflows_dir = "   "
active_workflow_id = "default"

[[workflows]]
id = "default"
name = "Default"
file = "default.toml"
"#;

        let error = parse_config(invalid).expect_err("whitespace workflows_dir should fail");
        assert!(
            error.to_string().contains("workflows_dir cannot be empty"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn rejects_duplicate_workflow_backing_files() {
        let invalid = r#"
listen = "127.0.0.1:9001"
admin_listen = "127.0.0.1:9002"
active_workflow_id = "default"

[[workflows]]
id = "default"
name = "Default"
file = "default.toml"

[[workflows]]
id = "secondary"
name = "Secondary"
file = "default.toml"
"#;

        let error = parse_config(invalid).expect_err("duplicate workflow file should fail");
        assert!(
            error
                .to_string()
                .contains("duplicate workflow file 'default.toml'"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn rejects_active_workflow_id_without_match() {
        let invalid = r#"
listen = "127.0.0.1:9001"
admin_listen = "127.0.0.1:9002"
active_workflow_id = "missing"

[[workflows]]
id = "default"
name = "Default"
file = "default.toml"
"#;

        let error = parse_config(invalid).expect_err("missing active workflow should fail");
        assert!(
            error
                .to_string()
                .contains("active_workflow_id 'missing' does not reference an indexed workflow"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn normalizes_legacy_route_provider_chain() {
        let legacy = r#"
listen = "127.0.0.1:9001"
admin_listen = "127.0.0.1:9002"

[[providers]]
id = "kimi"
name = "Kimi"
base_url = "https://api.kimi.com"

[[models]]
id = "kimi-k2"
name = "Kimi K2"
provider_id = "kimi"

[rule_graph]
version = 1
start_node_id = "start"

[[rule_graph.nodes]]
id = "start"
type = "start"
position = { x = 0.0, y = 0.0 }

[[rule_graph.nodes]]
id = "provider-kimi"
type = "route_provider"
position = { x = 120.0, y = 0.0 }

[rule_graph.nodes.route_provider]
provider_id = "kimi"

[[rule_graph.nodes]]
id = "model-kimi"
type = "select_model"
position = { x = 240.0, y = 0.0 }

[rule_graph.nodes.select_model]
provider_id = ""
model_id = "kimi-k2"

[[rule_graph.nodes]]
id = "end"
type = "end"
position = { x = 360.0, y = 0.0 }

[[rule_graph.edges]]
id = "edge-1"
source = "start"
target = "provider-kimi"

[[rule_graph.edges]]
id = "edge-2"
source = "provider-kimi"
target = "model-kimi"

[[rule_graph.edges]]
id = "edge-3"
source = "model-kimi"
target = "end"
"#;

        let config = parse_config(&legacy).expect("legacy config should normalize");
        let graph = config.rule_graph.expect("graph should exist");

        assert!(graph
            .nodes
            .iter()
            .all(|node| node.node_type != RuleGraphNodeType::RouteProvider));
        let select_model = graph
            .nodes
            .iter()
            .find(|node| node.id == "model-kimi")
            .and_then(|node| node.select_model.as_ref())
            .expect("select_model node should remain");
        assert_eq!(select_model.provider_id, "kimi");
        assert_eq!(select_model.model_id, "kimi-k2");
        assert!(
            graph
                .edges
                .iter()
                .any(|edge| edge.source == "start" && edge.target == "model-kimi"),
            "incoming edge should be rewired to the merged select_model node"
        );
    }

    #[test]
    fn rejects_missing_provider_reference() {
        let invalid = VALID_CONFIG.replace("provider_id = \"kimi\"", "provider_id = \"missing\"");
        let error = parse_config(&invalid).expect_err("config should reject missing provider");

        assert!(
            error
                .to_string()
                .contains("references missing provider 'missing'"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn rejects_global_rule_with_target() {
        let config = VALID_CONFIG.replace("scope = \"model\"", "scope = \"global\"");
        let error = parse_config(&config).expect_err("global scope should reject target_id");

        assert!(
            error
                .to_string()
                .contains("must not define target_id for global scope"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn rejects_rule_graph_with_cycle() {
        let invalid = format!(
            "{VALID_CONFIG}\n[[rule_graph.edges]]\nid = \"edge-4\"\nsource = \"end\"\ntarget = \"start\"\n"
        );
        let error = parse_config(&invalid).expect_err("cyclic rule graph should fail");
        assert!(
            error.to_string().contains("contains a cycle"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn parses_wasm_plugin_node() {
        let config =
            parse_config(VALID_WASM_PLUGIN_CONFIG).expect("wasm plugin config should parse");
        let graph = config.rule_graph.expect("graph should exist");
        let node = graph
            .nodes
            .iter()
            .find(|node| node.id == "plugin")
            .expect("plugin node should exist");

        assert_eq!(node.node_type, RuleGraphNodeType::WasmPlugin);
        let plugin = node
            .wasm_plugin
            .as_ref()
            .expect("wasm plugin config should exist");
        assert_eq!(plugin.plugin_id, "intent-classifier");
        assert_eq!(plugin.timeout_ms, 20);
        assert_eq!(plugin.fuel, Some(500000));
        assert_eq!(plugin.max_memory_bytes, 16_777_216);
        assert_eq!(plugin.granted_capabilities.len(), 2);
        assert_eq!(plugin.read_dirs, vec!["plugins-data/common"]);
        assert_eq!(plugin.write_dirs, vec!["plugins-data/runtime"]);
        assert_eq!(plugin.allowed_hosts, vec!["api.example.com:443"]);
        assert_eq!(
            plugin.config.get("prompt").and_then(|value| value.as_str()),
            Some("classify request intent")
        );
        assert_eq!(
            plugin
                .config
                .get("default_intent")
                .and_then(|value| value.as_str()),
            Some("chat")
        );
    }

    #[test]
    fn rejects_missing_plugin_id() {
        let invalid = VALID_WASM_PLUGIN_CONFIG.replace("plugin_id = \"intent-classifier\"\n", "");
        let error = parse_config(&invalid).expect_err("missing plugin_id should fail");

        assert!(
            error.to_string().contains("missing field `plugin_id`"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn parses_match_node() {
        let config = parse_config(VALID_WASM_MATCH_CONFIG).expect("wasm match config should parse");
        let graph = config.rule_graph.expect("graph should exist");
        let node = graph
            .nodes
            .iter()
            .find(|node| node.id == "matcher")
            .expect("matcher node should exist");

        assert_eq!(node.node_type, RuleGraphNodeType::Match);
        let plugin = node
            .match_node
            .as_ref()
            .expect("wasm match config should exist");
        assert_eq!(plugin.plugin.plugin_id, "remote-policy-router");
        assert_eq!(plugin.plugin.timeout_ms, 20);
        assert_eq!(plugin.plugin.fuel, Some(500000));
        assert_eq!(plugin.plugin.max_memory_bytes, 16_777_216);
        assert_eq!(
            plugin.plugin.granted_capabilities,
            vec![WasmCapability::Log]
        );
        assert_eq!(plugin.branches.len(), 1);
        assert_eq!(plugin.branches[0].id, "chat");
        assert_eq!(plugin.branches[0].expr, "ctx.header.x-tenant == enterprise");
        assert_eq!(plugin.branches[0].target_node_id, "end");
        assert_eq!(plugin.fallback_node_id.as_deref(), Some("end"));
        assert_eq!(
            plugin
                .plugin
                .config
                .get("match_header")
                .and_then(|value| value.as_str()),
            Some("x-tenant")
        );
    }

    #[test]
    fn parses_code_runner_node() {
        let config =
            parse_config(VALID_CODE_RUNNER_CONFIG).expect("code runner config should parse");
        let graph = config.rule_graph.expect("graph should exist");
        let node = graph
            .nodes
            .iter()
            .find(|node| node.id == "runner")
            .expect("runner node should exist");

        assert_eq!(node.node_type, RuleGraphNodeType::CodeRunner);
        let runner = node
            .code_runner
            .as_ref()
            .expect("code runner config should exist");
        assert_eq!(runner.language, CodeRunnerLanguage::Javascript);
        assert_eq!(runner.timeout_ms, 25);
        assert_eq!(runner.max_memory_bytes, 16_777_216);
        assert_eq!(runner.code, "export function run(input) { return {}; }");
    }

    #[test]
    fn rejects_empty_code_runner_code() {
        let invalid = VALID_CODE_RUNNER_CONFIG.replace(
            "code = \"export function run(input) { return {}; }\"",
            "code = \"   \"",
        );
        let error = parse_config(&invalid).expect_err("empty code should fail");
        let message = error.to_string();

        assert_eq!(message, "rule_graph node 'runner' code cannot be empty");
    }

    #[test]
    fn rejects_network_grant_without_allowed_hosts() {
        let invalid = VALID_WASM_PLUGIN_CONFIG.replace(
            "allowed_hosts = [\"api.example.com:443\"]\n",
            "allowed_hosts = []\n",
        );
        let error =
            parse_config(&invalid).expect_err("network grant without allowed hosts should fail");

        assert!(
            error
                .to_string()
                .contains("network capability requires allowed_hosts"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn accepts_valid_per_node_capability_grants() {
        let config =
            parse_config(VALID_WASM_PLUGIN_CONFIG).expect("valid capability grants should parse");
        let plugin = config
            .rule_graph
            .expect("graph should exist")
            .nodes
            .into_iter()
            .find(|node| node.id == "plugin")
            .and_then(|node| node.wasm_plugin)
            .expect("plugin node should exist");

        assert_eq!(plugin.granted_capabilities.len(), 2);
        assert_eq!(plugin.read_dirs, vec!["plugins-data/common"]);
        assert_eq!(plugin.write_dirs, vec!["plugins-data/runtime"]);
        assert_eq!(plugin.allowed_hosts, vec!["api.example.com:443"]);
    }

    #[test]
    fn rejects_parent_traversal_in_plugin_dirs() {
        let invalid = VALID_WASM_PLUGIN_CONFIG.replace(
            "read_dirs = [\"plugins-data/common\"]\n",
            "read_dirs = [\"plugins-data/../../secret\"]\n",
        );
        let error = parse_config(&invalid).expect_err("path traversal should fail");

        assert!(
            error
                .to_string()
                .contains("must not contain parent traversal"),
            "unexpected error: {error}"
        );
    }
}
