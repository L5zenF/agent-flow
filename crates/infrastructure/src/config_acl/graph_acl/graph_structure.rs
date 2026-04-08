use std::collections::HashSet;

use crate::config_acl::util::unique_ids;

use super::super::{
    InfrastructureAclError, RawRuleGraphConfig, RawRuleGraphEdge, RawRuleGraphNodeType,
};

pub(super) struct RuleGraphStructure<'a> {
    pub(super) graph: &'a RawRuleGraphConfig,
    node_ids: HashSet<&'a str>,
}

impl<'a> RuleGraphStructure<'a> {
    pub(super) fn new(graph: &'a RawRuleGraphConfig) -> Result<Self, InfrastructureAclError> {
        let node_ids = unique_ids(
            graph.nodes.iter().map(|node| node.id.as_str()),
            "rule_graph node",
        )?;
        unique_ids(
            graph.edges.iter().map(|edge| edge.id.as_str()),
            "rule_graph edge",
        )?;

        Ok(Self { graph, node_ids })
    }

    pub(super) fn validate(&self) -> Result<(), InfrastructureAclError> {
        self.validate_start_node()?;
        self.validate_edges()?;
        Ok(())
    }

    pub(super) fn contains_node(&self, node_id: &str) -> bool {
        self.node_ids.contains(node_id)
    }

    pub(super) fn outgoing_edge_count(&self, node_id: &str) -> usize {
        self.graph
            .edges
            .iter()
            .filter(|edge| edge.source == node_id)
            .count()
    }

    fn validate_start_node(&self) -> Result<(), InfrastructureAclError> {
        if self.graph.start_node_id.trim().is_empty() {
            return Err(InfrastructureAclError::Validation(
                "rule_graph start_node_id cannot be empty".to_string(),
            ));
        }
        if !self.contains_node(self.graph.start_node_id.as_str()) {
            return Err(InfrastructureAclError::Validation(format!(
                "rule_graph start node '{}' does not exist",
                self.graph.start_node_id
            )));
        }

        let start_count = self
            .graph
            .nodes
            .iter()
            .filter(|node| node.node_type == RawRuleGraphNodeType::Start)
            .count();
        if start_count != 1 {
            return Err(InfrastructureAclError::Validation(format!(
                "rule_graph requires exactly one start node, found {start_count}"
            )));
        }

        Ok(())
    }

    fn validate_edges(&self) -> Result<(), InfrastructureAclError> {
        for edge in &self.graph.edges {
            self.validate_edge(edge)?;
        }
        Ok(())
    }

    fn validate_edge(&self, edge: &RawRuleGraphEdge) -> Result<(), InfrastructureAclError> {
        if !self.contains_node(edge.source.as_str()) {
            return Err(InfrastructureAclError::Validation(format!(
                "rule_graph edge '{}' missing source '{}'",
                edge.id, edge.source
            )));
        }
        if !self.contains_node(edge.target.as_str()) {
            return Err(InfrastructureAclError::Validation(format!(
                "rule_graph edge '{}' missing target '{}'",
                edge.id, edge.target
            )));
        }
        Ok(())
    }
}
