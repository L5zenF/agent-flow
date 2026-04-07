use std::collections::{HashMap, HashSet};

use crate::ApplicationError;

#[derive(Debug, Clone)]
pub struct RuleGraphNodeValidationInput {
    pub id: String,
    pub is_start: bool,
}

#[derive(Debug, Clone)]
pub struct RuleGraphEdgeValidationInput {
    pub id: String,
    pub source: String,
    pub target: String,
}

#[derive(Debug, Clone)]
pub struct RuleGraphStructureInput {
    pub start_node_id: String,
    pub nodes: Vec<RuleGraphNodeValidationInput>,
    pub edges: Vec<RuleGraphEdgeValidationInput>,
}

pub fn validate_rule_graph_structure(
    input: &RuleGraphStructureInput,
) -> Result<HashSet<String>, ApplicationError> {
    if input.start_node_id.trim().is_empty() {
        return Err(ApplicationError::Validation(
            "rule_graph start_node_id cannot be empty".to_string(),
        ));
    }

    let node_ids = unique_ids(
        input.nodes.iter().map(|node| node.id.as_str()),
        "rule_graph node",
    )?;
    unique_ids(
        input.edges.iter().map(|edge| edge.id.as_str()),
        "rule_graph edge",
    )?;

    if !node_ids.contains(input.start_node_id.as_str()) {
        return Err(ApplicationError::Validation(format!(
            "rule_graph start node '{}' does not exist",
            input.start_node_id
        )));
    }

    let start_count = input.nodes.iter().filter(|node| node.is_start).count();
    if start_count != 1 {
        return Err(ApplicationError::Validation(format!(
            "rule_graph requires exactly one start node, found {start_count}"
        )));
    }

    for edge in &input.edges {
        if !node_ids.contains(edge.source.as_str()) {
            return Err(ApplicationError::Validation(format!(
                "rule_graph edge '{}' missing source '{}'",
                edge.id, edge.source
            )));
        }
        if !node_ids.contains(edge.target.as_str()) {
            return Err(ApplicationError::Validation(format!(
                "rule_graph edge '{}' missing target '{}'",
                edge.id, edge.target
            )));
        }
    }

    validate_acyclic(input)?;
    Ok(node_ids)
}

fn validate_acyclic(input: &RuleGraphStructureInput) -> Result<(), ApplicationError> {
    let adjacency = input
        .edges
        .iter()
        .fold(HashMap::<&str, Vec<&str>>::new(), |mut acc, edge| {
            acc.entry(edge.source.as_str())
                .or_default()
                .push(edge.target.as_str());
            acc
        });
    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();

    fn visit<'a>(
        node_id: &'a str,
        adjacency: &HashMap<&'a str, Vec<&'a str>>,
        visiting: &mut HashSet<&'a str>,
        visited: &mut HashSet<&'a str>,
    ) -> Result<(), ApplicationError> {
        if visited.contains(node_id) {
            return Ok(());
        }
        if !visiting.insert(node_id) {
            return Err(ApplicationError::Validation(format!(
                "rule_graph contains a cycle at node '{node_id}'"
            )));
        }

        if let Some(neighbors) = adjacency.get(node_id) {
            for target in neighbors {
                visit(target, adjacency, visiting, visited)?;
            }
        }
        visiting.remove(node_id);
        visited.insert(node_id);
        Ok(())
    }

    visit(
        input.start_node_id.as_str(),
        &adjacency,
        &mut visiting,
        &mut visited,
    )
}

fn unique_ids<'a>(
    ids: impl IntoIterator<Item = &'a str>,
    kind: &str,
) -> Result<HashSet<String>, ApplicationError> {
    let mut seen = HashSet::new();
    for id in ids {
        if id.trim().is_empty() {
            return Err(ApplicationError::Validation(format!(
                "{kind} id cannot be empty"
            )));
        }
        if !seen.insert(id.to_string()) {
            return Err(ApplicationError::Validation(format!(
                "duplicate {kind} id '{id}'"
            )));
        }
    }
    Ok(seen)
}

#[cfg(test)]
mod tests {
    use crate::{
        ApplicationError, RuleGraphEdgeValidationInput, RuleGraphNodeValidationInput,
        RuleGraphStructureInput, validate_rule_graph_structure,
    };

    fn valid_input() -> RuleGraphStructureInput {
        RuleGraphStructureInput {
            start_node_id: "start".to_string(),
            nodes: vec![
                RuleGraphNodeValidationInput {
                    id: "start".to_string(),
                    is_start: true,
                },
                RuleGraphNodeValidationInput {
                    id: "end".to_string(),
                    is_start: false,
                },
            ],
            edges: vec![RuleGraphEdgeValidationInput {
                id: "e1".to_string(),
                source: "start".to_string(),
                target: "end".to_string(),
            }],
        }
    }

    #[test]
    fn validates_rule_graph_structure() {
        validate_rule_graph_structure(&valid_input()).expect("structure should validate");
    }

    #[test]
    fn rejects_cycle() {
        let mut input = valid_input();
        input.edges.push(RuleGraphEdgeValidationInput {
            id: "e2".to_string(),
            source: "end".to_string(),
            target: "start".to_string(),
        });
        let error = validate_rule_graph_structure(&input).expect_err("cycle should fail");
        assert!(matches!(error, ApplicationError::Validation(_)));
    }
}
