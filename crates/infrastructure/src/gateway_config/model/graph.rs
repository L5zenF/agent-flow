use serde::{Deserialize, Serialize};

use super::{default_rule_graph_version, default_wasm_plugin_timeout_ms};

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
