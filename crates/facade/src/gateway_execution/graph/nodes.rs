use crate::config::{ConditionMode, RuleGraphNode, RuleGraphNodeType};
use crate::gateway_execution::context::{next_condition_edge, next_linear_edge};
use crate::gateway_execution::route_match::evaluate_router_clause;
use crate::rules::{evaluate_expression, render_template};

use super::state::GraphExecutionState;

impl<'cfg, 'env> GraphExecutionState<'cfg, 'env> {
    pub(super) fn execute_node(
        &mut self,
        node: &'env RuleGraphNode,
    ) -> Result<Option<&'env str>, String> {
        match node.node_type {
            RuleGraphNodeType::Start | RuleGraphNodeType::Note => {
                next_linear_edge(self.graph, self.current_id)
            }
            RuleGraphNodeType::Condition => self.execute_condition_node(node),
            RuleGraphNodeType::RouteProvider => self.execute_route_provider_node(node),
            RuleGraphNodeType::SelectModel => self.execute_select_model_node(node),
            RuleGraphNodeType::RewritePath => self.execute_rewrite_path_node(node),
            RuleGraphNodeType::SetContext => self.execute_set_context_node(node),
            RuleGraphNodeType::WasmPlugin => self.execute_wasm_plugin_node(node),
            RuleGraphNodeType::Match => self.execute_match_node(node),
            RuleGraphNodeType::CodeRunner => self.execute_code_runner_node(node),
            RuleGraphNodeType::Router => self.execute_router_node(node),
            RuleGraphNodeType::Log => self.execute_log_node(node),
            RuleGraphNodeType::SetHeader => self.execute_set_header_node(node),
            RuleGraphNodeType::RemoveHeader => self.execute_remove_header_node(node),
            RuleGraphNodeType::CopyHeader => self.execute_copy_header_node(node),
            RuleGraphNodeType::SetHeaderIfAbsent => self.execute_set_header_if_absent_node(node),
            RuleGraphNodeType::End => Ok(None),
        }
    }

    fn execute_condition_node(
        &self,
        node: &'env RuleGraphNode,
    ) -> Result<Option<&'env str>, String> {
        let condition = node
            .condition
            .as_ref()
            .ok_or_else(|| format!("rule_graph node '{}' missing condition config", node.id))?;
        let expression = match condition.mode {
            ConditionMode::Expression => condition
                .expression
                .clone()
                .ok_or_else(|| format!("rule_graph node '{}' missing expression", node.id))?,
            ConditionMode::Builder => {
                let builder = condition.builder.as_ref().ok_or_else(|| {
                    format!("rule_graph node '{}' missing builder config", node.id)
                })?;
                format!(
                    "{} {} \"{}\"",
                    builder.field, builder.operator, builder.value
                )
            }
        };
        let branch = if evaluate_expression(&expression, &self.request_context())? {
            "true"
        } else {
            "false"
        };
        next_condition_edge(self.graph, self.current_id, branch)
    }

    fn execute_route_provider_node(
        &mut self,
        node: &'env RuleGraphNode,
    ) -> Result<Option<&'env str>, String> {
        let provider_id = node
            .route_provider
            .as_ref()
            .ok_or_else(|| {
                format!(
                    "rule_graph node '{}' missing route_provider config",
                    node.id
                )
            })?
            .provider_id
            .as_str();
        self.selected_provider = Some(self.find_provider(provider_id)?);
        self.apply_selected_provider_headers()?;
        next_linear_edge(self.graph, self.current_id)
    }

    fn execute_select_model_node(
        &mut self,
        node: &'env RuleGraphNode,
    ) -> Result<Option<&'env str>, String> {
        let select_model = node
            .select_model
            .as_ref()
            .ok_or_else(|| format!("rule_graph node '{}' missing select_model config", node.id))?;
        self.selected_provider = Some(self.find_provider(select_model.provider_id.as_str())?);
        self.selected_model = Some(self.find_model(select_model.model_id.as_str())?);
        if let (Some(provider), Some(model)) = (self.selected_provider, self.selected_model) {
            if model.provider_id != provider.id {
                return Err(format!(
                    "rule_graph model '{}' does not belong to provider '{}'",
                    model.id, provider.id
                ));
            }
        }
        self.apply_selected_provider_headers()?;
        next_linear_edge(self.graph, self.current_id)
    }

    fn execute_rewrite_path_node(
        &mut self,
        node: &'env RuleGraphNode,
    ) -> Result<Option<&'env str>, String> {
        let node_config = node
            .rewrite_path
            .as_ref()
            .ok_or_else(|| format!("rule_graph node '{}' missing rewrite_path config", node.id))?;
        self.resolved_path = render_template(&node_config.value, &self.request_context())?;
        next_linear_edge(self.graph, self.current_id)
    }

    fn execute_set_context_node(
        &mut self,
        node: &'env RuleGraphNode,
    ) -> Result<Option<&'env str>, String> {
        let node_config = node
            .set_context
            .as_ref()
            .ok_or_else(|| format!("rule_graph node '{}' missing set_context config", node.id))?;
        let value = render_template(&node_config.value_template, &self.request_context())?;
        self.workflow_context.insert(node_config.key.clone(), value);
        next_linear_edge(self.graph, self.current_id)
    }

    fn execute_wasm_plugin_node(
        &mut self,
        node: &'env RuleGraphNode,
    ) -> Result<Option<&'env str>, String> {
        let node_config = node
            .wasm_plugin
            .as_ref()
            .ok_or_else(|| format!("rule_graph node '{}' missing wasm_plugin config", node.id))?;
        let port = self.executor.execute_wasm_runtime_node(
            node.id.as_str(),
            node_config,
            self.plugin_registry,
            self.method.as_str(),
            self.headers,
            self.selected_provider,
            self.selected_model,
            &mut self.resolved_path,
            &mut self.workflow_context,
            &mut self.outgoing_headers,
        )?;
        self.resolve_runtime_port(node.id.as_str(), node_config.plugin_id.as_str(), port)
    }

    fn execute_match_node(
        &mut self,
        node: &'env RuleGraphNode,
    ) -> Result<Option<&'env str>, String> {
        let node_config = node
            .match_node
            .as_ref()
            .ok_or_else(|| format!("rule_graph node '{}' missing match config", node.id))?;
        let mut matched_target = None;
        for branch in &node_config.branches {
            let mut branch_plugin_config = node_config.plugin.clone();
            branch_plugin_config
                .config
                .insert("expr".to_string(), toml::Value::String(branch.expr.clone()));
            branch_plugin_config.config.insert(
                "branch_id".to_string(),
                toml::Value::String(branch.id.clone()),
            );
            let port = self.executor.execute_wasm_runtime_node(
                node.id.as_str(),
                &branch_plugin_config,
                self.plugin_registry,
                self.method.as_str(),
                self.headers,
                self.selected_provider,
                self.selected_model,
                &mut self.resolved_path,
                &mut self.workflow_context,
                &mut self.outgoing_headers,
            )?;
            if matches!(port.as_deref(), Some("match")) {
                matched_target = Some(branch.target_node_id.as_str());
                break;
            }
        }
        Ok(matched_target.or(node_config.fallback_node_id.as_deref()))
    }

    fn execute_code_runner_node(
        &mut self,
        node: &'env RuleGraphNode,
    ) -> Result<Option<&'env str>, String> {
        let node_config = node
            .code_runner
            .as_ref()
            .ok_or_else(|| format!("rule_graph node '{}' missing code_runner config", node.id))?;
        let port = self.executor.execute_code_runner_node(
            self.plugin_registry,
            node.id.as_str(),
            node_config,
            self.method.as_str(),
            self.headers,
            self.selected_provider,
            self.selected_model,
            &mut self.resolved_path,
            &mut self.workflow_context,
            &mut self.outgoing_headers,
        )?;
        self.resolve_code_runner_port(node.id.as_str(), port)
    }

    fn execute_router_node(&self, node: &'env RuleGraphNode) -> Result<Option<&'env str>, String> {
        let node_config = node
            .router
            .as_ref()
            .ok_or_else(|| format!("rule_graph node '{}' missing router config", node.id))?;
        let request_context = self.request_context();
        let mut matched_target = None;
        for rule in &node_config.rules {
            if rule
                .clauses
                .iter()
                .all(|clause| evaluate_router_clause(clause, &request_context).unwrap_or(false))
            {
                matched_target = Some(rule.target_node_id.as_str());
                break;
            }
        }
        Ok(Some(
            matched_target
                .or(node_config.fallback_node_id.as_deref())
                .ok_or_else(|| {
                    format!(
                        "rule_graph router '{}' matched no rule and has no fallback target",
                        node.id
                    )
                })?,
        ))
    }

    fn execute_log_node(&self, node: &'env RuleGraphNode) -> Result<Option<&'env str>, String> {
        let node_config = node
            .log
            .as_ref()
            .ok_or_else(|| format!("rule_graph node '{}' missing log config", node.id))?;
        let message = render_template(&node_config.message, &self.request_context())?;
        tracing::info!(node_id = %node.id, message = %message, "rule graph log");
        next_linear_edge(self.graph, self.current_id)
    }

    fn execute_set_header_node(
        &mut self,
        node: &'env RuleGraphNode,
    ) -> Result<Option<&'env str>, String> {
        let node_config = node
            .set_header
            .as_ref()
            .ok_or_else(|| format!("rule_graph node '{}' missing set_header config", node.id))?;
        self.outgoing_headers.insert(
            node_config.name.to_ascii_lowercase(),
            vec![render_template(
                &node_config.value,
                &self.request_context(),
            )?],
        );
        next_linear_edge(self.graph, self.current_id)
    }

    fn execute_remove_header_node(
        &mut self,
        node: &'env RuleGraphNode,
    ) -> Result<Option<&'env str>, String> {
        let node_config = node
            .remove_header
            .as_ref()
            .ok_or_else(|| format!("rule_graph node '{}' missing remove_header config", node.id))?;
        self.outgoing_headers
            .remove(&node_config.name.to_ascii_lowercase());
        next_linear_edge(self.graph, self.current_id)
    }

    fn execute_copy_header_node(
        &mut self,
        node: &'env RuleGraphNode,
    ) -> Result<Option<&'env str>, String> {
        let node_config = node
            .copy_header
            .as_ref()
            .ok_or_else(|| format!("rule_graph node '{}' missing copy_header config", node.id))?;
        let source = self
            .headers
            .get(node_config.from.as_str())
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| {
                format!(
                    "header '{}' is unavailable for graph copy action",
                    node_config.from
                )
            })?;
        self.outgoing_headers.insert(
            node_config.to.to_ascii_lowercase(),
            vec![source.to_string()],
        );
        next_linear_edge(self.graph, self.current_id)
    }

    fn execute_set_header_if_absent_node(
        &mut self,
        node: &'env RuleGraphNode,
    ) -> Result<Option<&'env str>, String> {
        let node_config = node.set_header_if_absent.as_ref().ok_or_else(|| {
            format!(
                "rule_graph node '{}' missing set_header_if_absent config",
                node.id
            )
        })?;
        if !self
            .outgoing_headers
            .contains_key(&node_config.name.to_ascii_lowercase())
        {
            self.outgoing_headers.insert(
                node_config.name.to_ascii_lowercase(),
                vec![render_template(
                    &node_config.value,
                    &self.request_context(),
                )?],
            );
        }
        next_linear_edge(self.graph, self.current_id)
    }
}
