use std::collections::HashSet;

use super::{
    MatchBranchInput, RouterClauseInput, RouterRuleInput, validate_match_node,
    validate_route_provider_node, validate_router_node, validate_select_model_node,
};

#[test]
fn validates_router_node() {
    let node_ids = HashSet::from_iter(["a".to_string(), "b".to_string()]);
    let rules = vec![RouterRuleInput {
        id: "r1".to_string(),
        clauses: vec![RouterClauseInput {
            source: "ctx.intent".to_string(),
            operator: "eq".to_string(),
            value: "chat".to_string(),
        }],
        target_node_id: "b".to_string(),
    }];
    validate_router_node("a", &node_ids, Some(&rules), None).expect("router should validate");
}

#[test]
fn validates_match_node() {
    let node_ids = HashSet::from_iter(["a".to_string(), "b".to_string()]);
    let branches = vec![MatchBranchInput {
        id: "m1".to_string(),
        expr: "ctx.intent == \"chat\"".to_string(),
        target_node_id: "b".to_string(),
    }];
    validate_match_node("a", &node_ids, Some(&branches), None).expect("match should validate");
}

#[test]
fn validates_route_provider_and_select_model_nodes() {
    let provider_ids = HashSet::from_iter(["kimi".to_string()]);
    let model_ids = HashSet::from_iter(["kimi-k2".to_string()]);
    validate_route_provider_node("n1", Some("kimi"), &provider_ids)
        .expect("route provider should validate");
    validate_select_model_node(
        "n2",
        Some("kimi"),
        Some("kimi-k2"),
        &provider_ids,
        &model_ids,
        Some("kimi"),
    )
    .expect("select model should validate");
}
