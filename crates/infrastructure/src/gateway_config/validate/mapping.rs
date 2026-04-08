use application::{
    ConditionModeInput, GatewayValidationInput, HeaderRuleValidationInput, MatchBranchInput,
    ModelValidationInput, ProviderValidationInput, RouteValidationInput, RouterClauseInput,
    RouterRuleInput, RuleGraphEdgeValidationInput, RuleGraphNodeValidationInput,
    RuleGraphStructureInput, RuleScopeInput, WasmCapabilityInput, WorkflowValidationInput,
};

use crate::gateway_config::model::{
    ConditionMode, GatewayConfig, MatchNodeConfig, RouterNodeConfig, RuleGraphConfig,
    RuleGraphNodeType, RuleScope, WasmCapability, WasmPluginNodeConfig,
};

pub(super) fn map_gateway_validation_input(config: &GatewayConfig) -> GatewayValidationInput {
    GatewayValidationInput {
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
    }
}

pub(super) fn map_rule_graph_structure_input(graph: &RuleGraphConfig) -> RuleGraphStructureInput {
    RuleGraphStructureInput {
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
    }
}

pub(super) fn map_rule_scope(scope: RuleScope) -> RuleScopeInput {
    match scope {
        RuleScope::Global => RuleScopeInput::Global,
        RuleScope::Provider => RuleScopeInput::Provider,
        RuleScope::Model => RuleScopeInput::Model,
        RuleScope::Route => RuleScopeInput::Route,
    }
}

pub(super) fn map_condition_mode(mode: ConditionMode) -> ConditionModeInput {
    match mode {
        ConditionMode::Builder => ConditionModeInput::Builder,
        ConditionMode::Expression => ConditionModeInput::Expression,
    }
}

pub(super) fn map_wasm_capability(capability: WasmCapability) -> WasmCapabilityInput {
    match capability {
        WasmCapability::Log => WasmCapabilityInput::Log,
        WasmCapability::Fs => WasmCapabilityInput::Fs,
        WasmCapability::Network => WasmCapabilityInput::Network,
    }
}

pub(super) fn map_router_rules(config: Option<&RouterNodeConfig>) -> Option<Vec<RouterRuleInput>> {
    config.map(|config| {
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
            .collect()
    })
}

pub(super) fn map_match_branches(config: &MatchNodeConfig) -> Vec<MatchBranchInput> {
    config
        .branches
        .iter()
        .map(|branch| MatchBranchInput {
            id: branch.id.clone(),
            expr: branch.expr.clone(),
            target_node_id: branch.target_node_id.clone(),
        })
        .collect()
}

pub(super) fn map_wasm_grants(config: &WasmPluginNodeConfig) -> Vec<WasmCapabilityInput> {
    config
        .granted_capabilities
        .iter()
        .copied()
        .map(map_wasm_capability)
        .collect()
}
