use serde::{Deserialize, Serialize};

use super::{
    WorkflowIndexEntry, default_admin_listen, default_listen, routing::HeaderRuleConfig,
    routing::ModelConfig, routing::ProviderConfig, routing::RouteConfig,
};
use crate::gateway_config::model::RuleGraphConfig;

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
