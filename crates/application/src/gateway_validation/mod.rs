mod basics;
mod shared;
mod tests;
mod workflow_index;

use std::collections::HashSet;

pub use basics::validate_gateway_basics;

#[derive(Debug, Clone)]
pub struct ProviderValidationInput {
    pub id: String,
    pub base_url: String,
}

#[derive(Debug, Clone)]
pub struct ModelValidationInput {
    pub id: String,
    pub provider_id: String,
}

#[derive(Debug, Clone)]
pub struct RouteValidationInput {
    pub id: String,
    pub matcher: String,
    pub provider_id: String,
    pub model_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct HeaderRuleValidationInput {
    pub id: String,
    pub scope: crate::RuleScopeInput,
    pub target_id: Option<String>,
    pub actions_len: usize,
}

#[derive(Debug, Clone)]
pub struct WorkflowValidationInput {
    pub id: String,
    pub file: String,
}

#[derive(Debug, Clone)]
pub struct GatewayValidationInput {
    pub workflows_dir: Option<String>,
    pub active_workflow_id: Option<String>,
    pub providers: Vec<ProviderValidationInput>,
    pub models: Vec<ModelValidationInput>,
    pub routes: Vec<RouteValidationInput>,
    pub header_rules: Vec<HeaderRuleValidationInput>,
    pub workflows: Vec<WorkflowValidationInput>,
}

#[derive(Debug, Clone)]
pub struct GatewayValidationResult {
    pub provider_ids: HashSet<String>,
    pub model_ids: HashSet<String>,
    pub route_ids: HashSet<String>,
}
