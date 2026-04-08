use std::collections::{HashMap, HashSet};

use crate::gateway_config::model::{
    GatewayConfig, RuleGraphConfig, RuleGraphEdge, RuleGraphNode, RuleGraphNodeType,
    SelectModelNodeConfig, WorkflowIndexEntry,
};

pub fn normalize_legacy_rule_graph(mut config: GatewayConfig) -> GatewayConfig {
    let Some(graph) = config.rule_graph.take() else {
        return normalize_workflow_index_inputs(config);
    };

    let node_map = graph
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect::<HashMap<_, _>>();

    let incoming_edges = graph.edges.iter().fold(
        HashMap::<&str, Vec<&RuleGraphEdge>>::new(),
        |mut acc, edge| {
            acc.entry(edge.target.as_str()).or_default().push(edge);
            acc
        },
    );
    let outgoing_edges = graph.edges.iter().fold(
        HashMap::<&str, Vec<&RuleGraphEdge>>::new(),
        |mut acc, edge| {
            acc.entry(edge.source.as_str()).or_default().push(edge);
            acc
        },
    );

    let mut route_nodes_to_remove = HashSet::<String>::new();
    let mut updated_nodes = Vec::with_capacity(graph.nodes.len());
    let mut rewritten_edges = graph.edges.clone();

    for node in &graph.nodes {
        if node.node_type != RuleGraphNodeType::SelectModel {
            continue;
        }

        let Some(select_model) = &node.select_model else {
            continue;
        };
        if !select_model.provider_id.trim().is_empty() {
            continue;
        }

        let Some(incoming) = incoming_edges.get(node.id.as_str()) else {
            continue;
        };
        if incoming.len() != 1 {
            continue;
        }

        let route_edge = incoming[0];
        let Some(route_node) = node_map.get(route_edge.source.as_str()) else {
            continue;
        };
        if route_node.node_type != RuleGraphNodeType::RouteProvider {
            continue;
        }
        let Some(route_config) = route_node.route_provider.as_ref() else {
            continue;
        };

        let Some(route_outgoing) = outgoing_edges.get(route_node.id.as_str()) else {
            continue;
        };
        if route_outgoing.len() != 1 || route_outgoing[0].target != node.id {
            continue;
        }

        route_nodes_to_remove.insert(route_node.id.clone());
        updated_nodes.push(RuleGraphNode {
            select_model: Some(SelectModelNodeConfig {
                provider_id: route_config.provider_id.clone(),
                model_id: select_model.model_id.clone(),
            }),
            ..node.clone()
        });

        rewritten_edges = rewritten_edges
            .into_iter()
            .filter(|edge| edge.id != route_edge.id)
            .map(|edge| {
                if edge.target == route_node.id {
                    RuleGraphEdge {
                        target: node.id.clone(),
                        ..edge
                    }
                } else if edge.source == route_node.id {
                    RuleGraphEdge {
                        source: node.id.clone(),
                        ..edge
                    }
                } else {
                    edge
                }
            })
            .collect();
    }

    if route_nodes_to_remove.is_empty() {
        config.rule_graph = Some(graph);
        return normalize_workflow_index_inputs(synthesize_legacy_workflow_index(config));
    }

    let updated_node_ids = updated_nodes
        .iter()
        .map(|node| (node.id.clone(), node.clone()))
        .collect::<HashMap<_, _>>();

    config.rule_graph = Some(RuleGraphConfig {
        nodes: graph
            .nodes
            .into_iter()
            .filter(|node| !route_nodes_to_remove.contains(&node.id))
            .map(|node| updated_node_ids.get(&node.id).cloned().unwrap_or(node))
            .collect(),
        edges: rewritten_edges
            .into_iter()
            .filter(|edge| {
                !route_nodes_to_remove.contains(&edge.source)
                    && !route_nodes_to_remove.contains(&edge.target)
            })
            .collect(),
        ..graph
    });
    normalize_workflow_index_inputs(synthesize_legacy_workflow_index(config))
}

fn synthesize_legacy_workflow_index(mut config: GatewayConfig) -> GatewayConfig {
    if config.rule_graph.is_some() && config.workflows.is_empty() {
        let synthesized_workflow_id = config
            .active_workflow_id
            .clone()
            .unwrap_or_else(|| "default".to_string());

        config
            .workflows_dir
            .get_or_insert_with(|| "workflows".to_string());
        config
            .active_workflow_id
            .get_or_insert_with(|| synthesized_workflow_id.clone());
        config.workflows = vec![WorkflowIndexEntry {
            id: synthesized_workflow_id.clone(),
            name: if synthesized_workflow_id == "default" {
                "Default Workflow".to_string()
            } else {
                format!("Workflow {synthesized_workflow_id}")
            },
            file: format!("{synthesized_workflow_id}.toml"),
            description: Some("Migrated from legacy rule_graph".to_string()),
        }];
    }

    config
}

fn normalize_workflow_index_inputs(mut config: GatewayConfig) -> GatewayConfig {
    if let Some(workflows_dir) = config.workflows_dir.as_mut() {
        *workflows_dir = workflows_dir.trim().to_string();
    }
    for workflow in &mut config.workflows {
        workflow.file = workflow.file.trim().to_string();
    }

    config
}

pub(crate) fn uses_synthesized_legacy_workflow_index(config: &GatewayConfig) -> bool {
    if config.rule_graph.is_none() || config.workflows.len() != 1 {
        return false;
    }

    let Some(workflows_dir) = config.workflows_dir.as_deref() else {
        return false;
    };
    let Some(active_workflow_id) = config.active_workflow_id.as_deref() else {
        return false;
    };
    let workflow = &config.workflows[0];
    let expected_name = if active_workflow_id == "default" {
        "Default Workflow".to_string()
    } else {
        format!("Workflow {active_workflow_id}")
    };

    workflows_dir == "workflows"
        && workflow.id == active_workflow_id
        && workflow.name == expected_name
        && workflow.file == format!("{active_workflow_id}.toml")
        && workflow.description.as_deref() == Some("Migrated from legacy rule_graph")
}

pub(crate) fn is_not_found_error(error: &(dyn std::error::Error + 'static)) -> bool {
    error
        .downcast_ref::<std::io::Error>()
        .is_some_and(|io_error| io_error.kind() == std::io::ErrorKind::NotFound)
}
