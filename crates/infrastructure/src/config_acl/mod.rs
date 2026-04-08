mod error;
mod graph_acl;
mod mapping;
mod parser;
mod raw_model;
mod route_acl;
mod util;
mod workflow_acl;

use domain::{GatewayCatalog, WorkflowIndex};

pub use error::InfrastructureAclError;
pub use mapping::{map_gateway_catalog, map_workflow_index};
pub use parser::parse_raw_gateway_config;
pub use raw_model::{
    RawCodeRunnerNodeConfig, RawConditionBuilderConfig, RawConditionMode, RawConditionNodeConfig,
    RawCopyHeaderNodeConfig, RawGatewayConfig, RawHeaderMutationNodeConfig,
    RawHeaderNameNodeConfig, RawHeaderRule, RawLogNodeConfig, RawMatchBranchConfig,
    RawMatchNodeConfig, RawModel, RawProvider, RawRoute, RawRouterClause, RawRouterNodeConfig,
    RawRouterRule, RawRuleGraphConfig, RawRuleGraphEdge, RawRuleGraphNode, RawRuleGraphNodeType,
    RawRuleScope, RawSelectModelNodeConfig, RawWasmCapability, RawWasmPluginNodeConfig,
    RawWorkflowIndex, RawWorkflowIndexEntry,
};

pub fn map_gateway_config(
    raw: &RawGatewayConfig,
) -> Result<(GatewayCatalog, WorkflowIndex), InfrastructureAclError> {
    workflow_acl::validate_workflow_index_subset(raw)?;
    route_acl::validate_route_and_header_rule_subset(raw)?;
    if let Some(graph) = raw.rule_graph.as_ref() {
        graph_acl::validate_rule_graph_subset(graph, raw)?;
    }
    let gateway_catalog = map_gateway_catalog(&raw.providers, &raw.models)?;
    let workflow_index = map_workflow_index(&RawWorkflowIndex {
        workflows: raw.workflows.clone(),
        active_workflow_id: raw.active_workflow_id.clone(),
    })?;
    Ok((gateway_catalog, workflow_index))
}
