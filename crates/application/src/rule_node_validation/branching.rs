use std::collections::HashSet;

use crate::ApplicationError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionModeInput {
    Builder,
    Expression,
}

#[derive(Debug, Clone)]
pub struct RouterClauseInput {
    pub source: String,
    pub operator: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct RouterRuleInput {
    pub id: String,
    pub clauses: Vec<RouterClauseInput>,
    pub target_node_id: String,
}

#[derive(Debug, Clone)]
pub struct MatchBranchInput {
    pub id: String,
    pub expr: String,
    pub target_node_id: String,
}

pub fn validate_condition_node(
    node_id: &str,
    mode: ConditionModeInput,
    expression: Option<&str>,
    builder: Option<(&str, &str, &str)>,
    outgoing_count: usize,
) -> Result<(), ApplicationError> {
    match mode {
        ConditionModeInput::Expression => {
            if expression.unwrap_or("").trim().is_empty() {
                return Err(ApplicationError::Validation(format!(
                    "rule_graph condition node '{}' requires expression",
                    node_id
                )));
            }
        }
        ConditionModeInput::Builder => {
            let Some((field, operator, value)) = builder else {
                return Err(ApplicationError::Validation(format!(
                    "rule_graph condition node '{}' requires builder config",
                    node_id
                )));
            };
            if field.trim().is_empty() || operator.trim().is_empty() || value.trim().is_empty() {
                return Err(ApplicationError::Validation(format!(
                    "rule_graph condition node '{}' builder fields cannot be empty",
                    node_id
                )));
            }
        }
    }
    if outgoing_count > 2 {
        return Err(ApplicationError::Validation(format!(
            "rule_graph condition node '{}' supports at most 2 outgoing edges",
            node_id
        )));
    }
    Ok(())
}

pub fn validate_router_node(
    node_id: &str,
    node_ids: &HashSet<String>,
    rules: Option<&[RouterRuleInput]>,
    fallback_node_id: Option<&str>,
) -> Result<(), ApplicationError> {
    let Some(rules) = rules else {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' missing router config"
        )));
    };
    if rules.is_empty() {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' must define at least one router rule"
        )));
    }

    let mut rule_ids = HashSet::new();
    for rule in rules {
        if rule.id.trim().is_empty() {
            return Err(ApplicationError::Validation(format!(
                "rule_graph node '{node_id}' contains a router rule with empty id"
            )));
        }
        if !rule_ids.insert(rule.id.as_str()) {
            return Err(ApplicationError::Validation(format!(
                "rule_graph node '{node_id}' has duplicate router rule id '{}'",
                rule.id
            )));
        }
        if rule.clauses.is_empty() {
            return Err(ApplicationError::Validation(format!(
                "rule_graph node '{node_id}' router rule '{}' must contain at least one clause",
                rule.id
            )));
        }
        if rule.target_node_id.trim().is_empty() || !node_ids.contains(rule.target_node_id.as_str())
        {
            return Err(ApplicationError::Validation(format!(
                "rule_graph node '{node_id}' router rule '{}' references missing target '{}'",
                rule.id, rule.target_node_id
            )));
        }
        for clause in &rule.clauses {
            if clause.source.trim().is_empty()
                || clause.operator.trim().is_empty()
                || clause.value.trim().is_empty()
            {
                return Err(ApplicationError::Validation(format!(
                    "rule_graph node '{node_id}' router rule '{}' contains an incomplete clause",
                    rule.id
                )));
            }
        }
    }

    if let Some(fallback) = fallback_node_id {
        if fallback.trim().is_empty() || !node_ids.contains(fallback) {
            return Err(ApplicationError::Validation(format!(
                "rule_graph node '{node_id}' references missing fallback target '{fallback}'"
            )));
        }
    }

    Ok(())
}

pub fn validate_match_node(
    node_id: &str,
    node_ids: &HashSet<String>,
    branches: Option<&[MatchBranchInput]>,
    fallback_node_id: Option<&str>,
) -> Result<(), ApplicationError> {
    let Some(branches) = branches else {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' missing match config"
        )));
    };
    if branches.is_empty() {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' must define at least one match branch"
        )));
    }

    let mut branch_ids = HashSet::new();
    for branch in branches {
        if branch.id.trim().is_empty() {
            return Err(ApplicationError::Validation(format!(
                "rule_graph node '{node_id}' contains a match branch with empty id"
            )));
        }
        if !branch_ids.insert(branch.id.as_str()) {
            return Err(ApplicationError::Validation(format!(
                "rule_graph node '{node_id}' has duplicate match branch id '{}'",
                branch.id
            )));
        }
        if branch.expr.trim().is_empty() {
            return Err(ApplicationError::Validation(format!(
                "rule_graph node '{node_id}' match branch '{}' has empty expr",
                branch.id
            )));
        }
        if branch.target_node_id.trim().is_empty()
            || !node_ids.contains(branch.target_node_id.as_str())
        {
            return Err(ApplicationError::Validation(format!(
                "rule_graph node '{node_id}' match branch '{}' references missing target '{}'",
                branch.id, branch.target_node_id
            )));
        }
    }

    if let Some(fallback) = fallback_node_id {
        if fallback.trim().is_empty() || !node_ids.contains(fallback) {
            return Err(ApplicationError::Validation(format!(
                "rule_graph node '{node_id}' references missing fallback target '{fallback}'"
            )));
        }
    }

    Ok(())
}
