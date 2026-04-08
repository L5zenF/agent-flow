use std::collections::HashSet;

use super::super::{
    InfrastructureAclError, RawCodeRunnerNodeConfig, RawConditionMode, RawConditionNodeConfig,
    RawGatewayConfig, RawMatchNodeConfig, RawRouterNodeConfig, RawRuleGraphConfig,
    RawRuleGraphNode, RawRuleGraphNodeType,
};
use super::graph_structure::RuleGraphStructure;
use super::wasm_plugin_acl::WasmPluginAcl;

pub(super) struct RuleGraphNodeAcl<'a> {
    graph: &'a RawRuleGraphConfig,
    raw: &'a RawGatewayConfig,
    structure: &'a RuleGraphStructure<'a>,
    provider_ids: HashSet<&'a str>,
    model_ids: HashSet<&'a str>,
}

impl<'a> RuleGraphNodeAcl<'a> {
    pub(super) fn new(
        graph: &'a RawRuleGraphConfig,
        raw: &'a RawGatewayConfig,
        structure: &'a RuleGraphStructure<'a>,
    ) -> Self {
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

        Self {
            graph,
            raw,
            structure,
            provider_ids,
            model_ids,
        }
    }

    pub(super) fn validate(&self) -> Result<(), InfrastructureAclError> {
        for node in &self.graph.nodes {
            self.validate_node(node)?;
        }
        Ok(())
    }

    fn validate_node(&self, node: &RawRuleGraphNode) -> Result<(), InfrastructureAclError> {
        match node.node_type {
            RawRuleGraphNodeType::Condition => {
                self.validate_condition_node(node.id.as_str(), node.condition.as_ref())
            }
            RawRuleGraphNodeType::SelectModel => self.validate_select_model_node(node),
            RawRuleGraphNodeType::Log => self.validate_log_node(node),
            RawRuleGraphNodeType::SetHeader | RawRuleGraphNodeType::SetHeaderIfAbsent => {
                self.validate_set_header_node(node)
            }
            RawRuleGraphNodeType::RemoveHeader => self.validate_remove_header_node(node),
            RawRuleGraphNodeType::CopyHeader => self.validate_copy_header_node(node),
            RawRuleGraphNodeType::Router => {
                self.validate_router_node(node.id.as_str(), node.router.as_ref())
            }
            RawRuleGraphNodeType::WasmPlugin => {
                WasmPluginAcl::new(node.id.as_str(), node.wasm_plugin.as_ref())?.validate()
            }
            RawRuleGraphNodeType::Match => {
                self.validate_match_node(node.id.as_str(), node.match_node.as_ref())
            }
            RawRuleGraphNodeType::CodeRunner => {
                self.validate_code_runner_node(node.id.as_str(), node.code_runner.as_ref())
            }
            _ => Ok(()),
        }
    }

    fn validate_condition_node(
        &self,
        node_id: &str,
        condition: Option<&RawConditionNodeConfig>,
    ) -> Result<(), InfrastructureAclError> {
        let Some(condition) = condition else {
            return Err(InfrastructureAclError::Validation(format!(
                "rule_graph node '{}' missing condition config",
                node_id
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
                        node_id
                    )));
                }
            }
            RawConditionMode::Builder => {
                let Some(builder) = &condition.builder else {
                    return Err(InfrastructureAclError::Validation(format!(
                        "rule_graph condition node '{}' requires builder config",
                        node_id
                    )));
                };
                if builder.field.trim().is_empty()
                    || builder.operator.trim().is_empty()
                    || builder.value.trim().is_empty()
                {
                    return Err(InfrastructureAclError::Validation(format!(
                        "rule_graph condition node '{}' builder fields cannot be empty",
                        node_id
                    )));
                }
            }
        }
        if self.structure.outgoing_edge_count(node_id) > 2 {
            return Err(InfrastructureAclError::Validation(format!(
                "rule_graph condition node '{}' supports at most 2 outgoing edges",
                node_id
            )));
        }
        Ok(())
    }

    fn validate_select_model_node(
        &self,
        node: &RawRuleGraphNode,
    ) -> Result<(), InfrastructureAclError> {
        let Some(config) = &node.select_model else {
            return Err(InfrastructureAclError::Validation(format!(
                "rule_graph node '{}' missing select_model config",
                node.id
            )));
        };
        if !self.provider_ids.contains(config.provider_id.as_str()) {
            return Err(InfrastructureAclError::Validation(format!(
                "rule_graph node '{}' references missing provider '{}'",
                node.id, config.provider_id
            )));
        }
        if !self.model_ids.contains(config.model_id.as_str()) {
            return Err(InfrastructureAclError::Validation(format!(
                "rule_graph node '{}' references missing model '{}'",
                node.id, config.model_id
            )));
        }
        let Some(model) = self
            .raw
            .models
            .iter()
            .find(|model| model.id == config.model_id)
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
        Ok(())
    }

    fn validate_log_node(&self, node: &RawRuleGraphNode) -> Result<(), InfrastructureAclError> {
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
        Ok(())
    }

    fn validate_set_header_node(
        &self,
        node: &RawRuleGraphNode,
    ) -> Result<(), InfrastructureAclError> {
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
        Ok(())
    }

    fn validate_remove_header_node(
        &self,
        node: &RawRuleGraphNode,
    ) -> Result<(), InfrastructureAclError> {
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
        Ok(())
    }

    fn validate_copy_header_node(
        &self,
        node: &RawRuleGraphNode,
    ) -> Result<(), InfrastructureAclError> {
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
        Ok(())
    }

    fn validate_router_node(
        &self,
        node_id: &str,
        config: Option<&RawRouterNodeConfig>,
    ) -> Result<(), InfrastructureAclError> {
        let Some(config) = config else {
            return Err(InfrastructureAclError::Validation(format!(
                "rule_graph node '{}' missing router config",
                node_id
            )));
        };
        if config.rules.is_empty() {
            return Err(InfrastructureAclError::Validation(format!(
                "rule_graph node '{}' must define at least one router rule",
                node_id
            )));
        }

        let mut rule_ids = HashSet::new();
        for rule in &config.rules {
            if rule.id.trim().is_empty() {
                return Err(InfrastructureAclError::Validation(format!(
                    "rule_graph node '{}' contains a router rule with empty id",
                    node_id
                )));
            }
            if !rule_ids.insert(rule.id.as_str()) {
                return Err(InfrastructureAclError::Validation(format!(
                    "rule_graph node '{}' has duplicate router rule id '{}'",
                    node_id, rule.id
                )));
            }
            if rule.clauses.is_empty() {
                return Err(InfrastructureAclError::Validation(format!(
                    "rule_graph node '{}' router rule '{}' must contain at least one clause",
                    node_id, rule.id
                )));
            }
            if !self.has_target(rule.target_node_id.as_str()) {
                return Err(InfrastructureAclError::Validation(format!(
                    "rule_graph node '{}' router rule '{}' references missing target '{}'",
                    node_id, rule.id, rule.target_node_id
                )));
            }
            for clause in &rule.clauses {
                if clause.source.trim().is_empty()
                    || clause.operator.trim().is_empty()
                    || clause.value.trim().is_empty()
                {
                    return Err(InfrastructureAclError::Validation(format!(
                        "rule_graph node '{}' router rule '{}' contains an incomplete clause",
                        node_id, rule.id
                    )));
                }
            }
        }

        self.validate_optional_target(node_id, config.fallback_node_id.as_deref())
    }

    fn validate_match_node(
        &self,
        node_id: &str,
        config: Option<&RawMatchNodeConfig>,
    ) -> Result<(), InfrastructureAclError> {
        let Some(config) = config else {
            return Err(InfrastructureAclError::Validation(format!(
                "rule_graph node '{node_id}' missing match config"
            )));
        };
        WasmPluginAcl::from_config(node_id, &config.plugin).validate()?;

        if config.branches.is_empty() {
            return Err(InfrastructureAclError::Validation(format!(
                "rule_graph node '{node_id}' must define at least one match branch"
            )));
        }

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
            if !self.has_target(branch.target_node_id.as_str()) {
                return Err(InfrastructureAclError::Validation(format!(
                    "rule_graph node '{node_id}' match branch '{}' references missing target '{}'",
                    branch.id, branch.target_node_id
                )));
            }
        }

        self.validate_optional_target(node_id, config.fallback_node_id.as_deref())
    }

    fn validate_code_runner_node(
        &self,
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

    fn has_target(&self, node_id: &str) -> bool {
        !node_id.trim().is_empty() && self.structure.contains_node(node_id)
    }

    fn validate_optional_target(
        &self,
        node_id: &str,
        fallback_node_id: Option<&str>,
    ) -> Result<(), InfrastructureAclError> {
        if let Some(fallback) = fallback_node_id
            && !self.has_target(fallback)
        {
            return Err(InfrastructureAclError::Validation(format!(
                "rule_graph node '{}' references missing fallback target '{}'",
                node_id, fallback
            )));
        }
        Ok(())
    }
}
