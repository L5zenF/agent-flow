use serde::Deserialize;

use super::util::default_wasm_plugin_timeout_ms;

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
