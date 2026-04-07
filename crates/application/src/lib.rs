mod config_admin;
mod error;
mod gateway_config;
mod gateway_validation;
mod header_policy;
mod rule_graph_validation;
mod rule_node_validation;
mod settings_schema;
mod workflow_admin;
mod workflow_runtime;

pub use crate::config_admin::{reload_runtime_state, replace_config, validate_candidate_config};
pub use crate::error::ApplicationError;
pub use crate::gateway_config::{
    GatewaySummary, summarize_gateway_catalog, summarize_gateway_from_source,
};
pub use crate::gateway_validation::{
    GatewayValidationInput, GatewayValidationResult, HeaderRuleValidationInput,
    ModelValidationInput, ProviderValidationInput, RouteValidationInput, WorkflowValidationInput,
    validate_gateway_basics,
};
pub use crate::header_policy::{
    HeaderActionInput, HeaderRuleInput, HeaderValueInput, PolicyRequestFacts,
    ProviderDefaultHeaderInput, RuleScopeInput, evaluate_expression as evaluate_policy_expression,
    render_template as render_policy_template, resolve_headers as resolve_policy_headers,
};
pub use crate::rule_graph_validation::{
    RuleGraphEdgeValidationInput, RuleGraphNodeValidationInput, RuleGraphStructureInput,
    validate_rule_graph_structure,
};
pub use crate::rule_node_validation::{
    ConditionModeInput, MatchBranchInput, RouterClauseInput, RouterRuleInput, WasmCapabilityInput,
    validate_code_runner_node as validate_rule_code_runner_node,
    validate_condition_node as validate_rule_condition_node,
    validate_copy_header_node as validate_rule_copy_header_node,
    validate_header_mutation_node as validate_rule_header_mutation_node,
    validate_header_name_node as validate_rule_header_name_node,
    validate_log_node as validate_rule_log_node, validate_match_node as validate_rule_match_node,
    validate_route_provider_node as validate_rule_route_provider_node,
    validate_router_node as validate_rule_router_node,
    validate_select_model_node as validate_rule_select_model_node,
    validate_set_context_node as validate_rule_set_context_node,
    validate_value_node as validate_rule_value_node,
    validate_wasm_plugin_node as validate_rule_wasm_plugin_node,
};
pub use crate::settings_schema::{
    SettingsSchema, SettingsSchemaField, SettingsSchemaSection, gateway_settings_schema,
};
pub use crate::workflow_admin::{
    WorkflowAdminError, WorkflowCreatePlan, WorkflowEntryInput, plan_create_workflow,
    require_workflow,
};
pub use crate::workflow_runtime::{
    WorkflowIndexEntryInput, ensure_active_workflow_loaded, load_indexed_workflows,
    resolve_workflow_path as resolve_indexed_workflow_path,
    resolve_workflows_dir as resolve_indexed_workflows_dir,
};
