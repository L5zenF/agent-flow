use std::collections::HashSet;
use std::path::{Component, Path};

use crate::ApplicationError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionModeInput {
    Builder,
    Expression,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WasmCapabilityInput {
    Log,
    Fs,
    Network,
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

pub fn validate_route_provider_node(
    node_id: &str,
    provider_id: Option<&str>,
    provider_ids: &HashSet<String>,
) -> Result<(), ApplicationError> {
    let Some(provider_id) = provider_id else {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{}' missing route_provider config",
            node_id
        )));
    };
    if !provider_ids.contains(provider_id) {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{}' references missing provider '{}'",
            node_id, provider_id
        )));
    }
    Ok(())
}

pub fn validate_select_model_node(
    node_id: &str,
    provider_id: Option<&str>,
    model_id: Option<&str>,
    provider_ids: &HashSet<String>,
    model_ids: &HashSet<String>,
    model_provider_id: Option<&str>,
) -> Result<(), ApplicationError> {
    let Some(provider_id) = provider_id else {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{}' missing select_model config",
            node_id
        )));
    };
    let Some(model_id) = model_id else {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{}' missing select_model config",
            node_id
        )));
    };
    if !provider_ids.contains(provider_id) {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{}' references missing provider '{}'",
            node_id, provider_id
        )));
    }
    if !model_ids.contains(model_id) {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{}' references missing model '{}'",
            node_id, model_id
        )));
    }
    let Some(model_provider_id) = model_provider_id else {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{}' references missing model '{}'",
            node_id, model_id
        )));
    };
    if model_provider_id != provider_id {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{}' model '{}' does not belong to provider '{}'",
            node_id, model_id, provider_id
        )));
    }
    Ok(())
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

pub fn validate_value_node(node_id: &str, value: Option<&str>) -> Result<(), ApplicationError> {
    let Some(value) = value else {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' missing value config"
        )));
    };
    if value.trim().is_empty() {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' value cannot be empty"
        )));
    }
    Ok(())
}

pub fn validate_set_context_node(
    node_id: &str,
    key: Option<&str>,
    value_template: Option<&str>,
) -> Result<(), ApplicationError> {
    let Some(key) = key else {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' missing set_context config"
        )));
    };
    let Some(value_template) = value_template else {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' missing set_context config"
        )));
    };
    if key.trim().is_empty() {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' context key cannot be empty"
        )));
    }
    if value_template.trim().is_empty() {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' context value_template cannot be empty"
        )));
    }
    Ok(())
}

pub fn validate_log_node(node_id: &str, message: Option<&str>) -> Result<(), ApplicationError> {
    let Some(message) = message else {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' missing log config"
        )));
    };
    if message.trim().is_empty() {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' log message cannot be empty"
        )));
    }
    Ok(())
}

pub fn validate_header_mutation_node(
    node_id: &str,
    name: Option<&str>,
    value: Option<&str>,
) -> Result<(), ApplicationError> {
    let Some(name) = name else {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' missing header config"
        )));
    };
    let Some(value) = value else {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' missing header config"
        )));
    };
    if name.trim().is_empty() || value.trim().is_empty() {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' header name/value cannot be empty"
        )));
    }
    Ok(())
}

pub fn validate_header_name_node(
    node_id: &str,
    name: Option<&str>,
) -> Result<(), ApplicationError> {
    let Some(name) = name else {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' missing remove_header config"
        )));
    };
    if name.trim().is_empty() {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' header name cannot be empty"
        )));
    }
    Ok(())
}

pub fn validate_copy_header_node(
    node_id: &str,
    from: Option<&str>,
    to: Option<&str>,
) -> Result<(), ApplicationError> {
    let Some(from) = from else {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' missing copy_header config"
        )));
    };
    let Some(to) = to else {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' missing copy_header config"
        )));
    };
    if from.trim().is_empty() || to.trim().is_empty() {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' copy header fields cannot be empty"
        )));
    }
    Ok(())
}

pub fn validate_code_runner_node(
    node_id: &str,
    timeout_ms: Option<u64>,
    max_memory_bytes: Option<u64>,
    code: Option<&str>,
) -> Result<(), ApplicationError> {
    let Some(timeout_ms) = timeout_ms else {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' missing code_runner config"
        )));
    };
    let Some(max_memory_bytes) = max_memory_bytes else {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' missing code_runner config"
        )));
    };
    let Some(code) = code else {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' missing code_runner config"
        )));
    };

    if timeout_ms == 0 {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' timeout_ms must be greater than zero"
        )));
    }
    if max_memory_bytes == 0 {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' max_memory_bytes must be greater than zero"
        )));
    }
    if code.trim().is_empty() {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' code cannot be empty"
        )));
    }
    Ok(())
}

pub fn validate_wasm_plugin_node(
    node_id: &str,
    plugin_id: Option<&str>,
    timeout_ms: Option<u64>,
    fuel: Option<Option<u64>>,
    max_memory_bytes: Option<u64>,
    granted_capabilities: &[WasmCapabilityInput],
    read_dirs: &[String],
    write_dirs: &[String],
    allowed_hosts: &[String],
) -> Result<(), ApplicationError> {
    let Some(plugin_id) = plugin_id else {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' missing wasm_plugin config"
        )));
    };
    let Some(timeout_ms) = timeout_ms else {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' missing wasm_plugin config"
        )));
    };
    let Some(fuel) = fuel else {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' missing wasm_plugin config"
        )));
    };
    let Some(max_memory_bytes) = max_memory_bytes else {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' missing wasm_plugin config"
        )));
    };

    if plugin_id.trim().is_empty() {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' plugin_id cannot be empty"
        )));
    }
    if timeout_ms == 0 {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' timeout_ms must be greater than zero"
        )));
    }
    if matches!(fuel, Some(0)) {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' fuel must be greater than zero when set"
        )));
    }
    if max_memory_bytes == 0 {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' max_memory_bytes must be greater than zero"
        )));
    }

    let grants = granted_capabilities.iter().copied().collect::<HashSet<_>>();
    let has_fs = grants.contains(&WasmCapabilityInput::Fs);
    let has_network = grants.contains(&WasmCapabilityInput::Network);

    if has_fs {
        if read_dirs.is_empty() && write_dirs.is_empty() {
            return Err(ApplicationError::Validation(format!(
                "rule_graph node '{node_id}' fs capability requires read_dirs or write_dirs"
            )));
        }
    } else if !read_dirs.is_empty() || !write_dirs.is_empty() {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' fs directories require an fs capability grant"
        )));
    }

    validate_wasm_paths(node_id, "read_dirs", read_dirs)?;
    validate_wasm_paths(node_id, "write_dirs", write_dirs)?;

    if has_network {
        if allowed_hosts.is_empty() {
            return Err(ApplicationError::Validation(format!(
                "rule_graph node '{node_id}' network capability requires allowed_hosts"
            )));
        }
    } else if !allowed_hosts.is_empty() {
        return Err(ApplicationError::Validation(format!(
            "rule_graph node '{node_id}' allowed_hosts require a network capability grant"
        )));
    }

    validate_wasm_hosts(node_id, allowed_hosts)?;
    Ok(())
}

fn validate_wasm_paths(
    node_id: &str,
    field: &str,
    paths: &[String],
) -> Result<(), ApplicationError> {
    for path in paths {
        if path.trim().is_empty() {
            return Err(ApplicationError::Validation(format!(
                "rule_graph node '{node_id}' {field} cannot contain empty paths"
            )));
        }
        let path_ref = Path::new(path);
        if path_ref.is_absolute() {
            return Err(ApplicationError::Validation(format!(
                "rule_graph node '{node_id}' {field} must use relative paths"
            )));
        }
        if path_ref.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::Prefix(_) | Component::RootDir
            )
        }) {
            return Err(ApplicationError::Validation(format!(
                "rule_graph node '{node_id}' {field} must not contain parent traversal"
            )));
        }
    }
    Ok(())
}

fn validate_wasm_hosts(node_id: &str, hosts: &[String]) -> Result<(), ApplicationError> {
    for host in hosts {
        if host.trim().is_empty() {
            return Err(ApplicationError::Validation(format!(
                "rule_graph node '{node_id}' allowed_hosts cannot contain empty hosts"
            )));
        }
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

#[cfg(test)]
mod tests {
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
}
