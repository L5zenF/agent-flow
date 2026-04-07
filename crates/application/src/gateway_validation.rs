use std::collections::HashSet;
use std::path::{Component, Path};

use crate::{ApplicationError, RuleScopeInput};

#[derive(Debug, Clone)]
pub struct ProviderValidationInput {
    pub id: String,
    pub base_url: String,
}

#[derive(Debug, Clone)]
pub struct ModelValidationInput {
    pub id: String,
    pub provider_id: String,
}

#[derive(Debug, Clone)]
pub struct RouteValidationInput {
    pub id: String,
    pub matcher: String,
    pub provider_id: String,
    pub model_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct HeaderRuleValidationInput {
    pub id: String,
    pub scope: RuleScopeInput,
    pub target_id: Option<String>,
    pub actions_len: usize,
}

#[derive(Debug, Clone)]
pub struct WorkflowValidationInput {
    pub id: String,
    pub file: String,
}

#[derive(Debug, Clone)]
pub struct GatewayValidationInput {
    pub workflows_dir: Option<String>,
    pub active_workflow_id: Option<String>,
    pub providers: Vec<ProviderValidationInput>,
    pub models: Vec<ModelValidationInput>,
    pub routes: Vec<RouteValidationInput>,
    pub header_rules: Vec<HeaderRuleValidationInput>,
    pub workflows: Vec<WorkflowValidationInput>,
}

#[derive(Debug, Clone)]
pub struct GatewayValidationResult {
    pub provider_ids: HashSet<String>,
    pub model_ids: HashSet<String>,
    pub route_ids: HashSet<String>,
}

pub fn validate_gateway_basics(
    input: &GatewayValidationInput,
) -> Result<GatewayValidationResult, ApplicationError> {
    let provider_ids = unique_ids(
        input.providers.iter().map(|provider| provider.id.as_str()),
        "provider",
    )?;
    let model_ids = unique_ids(input.models.iter().map(|model| model.id.as_str()), "model")?;
    let route_ids = unique_ids(input.routes.iter().map(|route| route.id.as_str()), "route")?;
    unique_ids(
        input.header_rules.iter().map(|rule| rule.id.as_str()),
        "header_rule",
    )?;

    for provider in &input.providers {
        if provider.id.trim().is_empty() {
            return Err(ApplicationError::Validation(
                "provider id cannot be empty".to_string(),
            ));
        }
        if provider.base_url.trim().is_empty() {
            return Err(ApplicationError::Validation(format!(
                "provider '{}' base_url cannot be empty",
                provider.id
            )));
        }
    }

    for model in &input.models {
        if !provider_ids.contains(model.provider_id.as_str()) {
            return Err(ApplicationError::Validation(format!(
                "model '{}' references missing provider '{}'",
                model.id, model.provider_id
            )));
        }
    }

    for route in &input.routes {
        if route.matcher.trim().is_empty() {
            return Err(ApplicationError::Validation(format!(
                "route '{}' matcher cannot be empty",
                route.id
            )));
        }
        if !provider_ids.contains(route.provider_id.as_str()) {
            return Err(ApplicationError::Validation(format!(
                "route '{}' references missing provider '{}'",
                route.id, route.provider_id
            )));
        }
        if let Some(model_id) = route.model_id.as_deref() {
            if !model_ids.contains(model_id) {
                return Err(ApplicationError::Validation(format!(
                    "route '{}' references missing model '{}'",
                    route.id, model_id
                )));
            }
        }
    }

    for rule in &input.header_rules {
        match rule.scope {
            RuleScopeInput::Global => {
                if rule.target_id.is_some() {
                    return Err(ApplicationError::Validation(format!(
                        "header_rule '{}' must not define target_id for global scope",
                        rule.id
                    )));
                }
            }
            RuleScopeInput::Provider => {
                validate_rule_target(rule, &provider_ids, "provider")?;
            }
            RuleScopeInput::Model => {
                validate_rule_target(rule, &model_ids, "model")?;
            }
            RuleScopeInput::Route => {
                validate_rule_target(rule, &route_ids, "route")?;
            }
        }

        if rule.actions_len == 0 {
            return Err(ApplicationError::Validation(format!(
                "header_rule '{}' must contain at least one action",
                rule.id
            )));
        }
    }

    validate_workflow_index(
        input.workflows_dir.as_deref(),
        input.active_workflow_id.as_deref(),
        &input.workflows,
    )?;

    Ok(GatewayValidationResult {
        provider_ids,
        model_ids,
        route_ids,
    })
}

fn validate_rule_target(
    rule: &HeaderRuleValidationInput,
    ids: &HashSet<String>,
    kind: &str,
) -> Result<(), ApplicationError> {
    let Some(target_id) = rule.target_id.as_deref() else {
        return Err(ApplicationError::Validation(format!(
            "header_rule '{}' requires target_id for {kind} scope",
            rule.id
        )));
    };

    if !ids.contains(target_id) {
        return Err(ApplicationError::Validation(format!(
            "header_rule '{}' references missing {kind} '{}'",
            rule.id, target_id
        )));
    }

    Ok(())
}

fn validate_workflow_index(
    workflows_dir: Option<&str>,
    active_workflow_id: Option<&str>,
    workflows: &[WorkflowValidationInput],
) -> Result<(), ApplicationError> {
    if let Some(workflows_dir) = workflows_dir {
        validate_workflow_relative_path("workflows_dir", workflows_dir)?;
    }

    let workflow_ids = unique_ids(
        workflows.iter().map(|workflow| workflow.id.as_str()),
        "workflow",
    )?;

    let mut workflow_files = HashSet::new();
    for workflow in workflows {
        if !workflow_files.insert(workflow.file.as_str()) {
            return Err(ApplicationError::Validation(format!(
                "duplicate workflow file '{}'",
                workflow.file
            )));
        }
    }

    for workflow in workflows {
        if workflow.file.trim().is_empty() {
            return Err(ApplicationError::Validation(format!(
                "workflow '{}' file cannot be empty",
                workflow.id
            )));
        }
        validate_workflow_relative_path(
            &format!("workflow '{}' file", workflow.id),
            &workflow.file,
        )?;
    }

    if !workflows.is_empty() {
        let Some(active_workflow_id) = active_workflow_id else {
            return Err(ApplicationError::Validation(
                "active_workflow_id must be set when workflows are present".to_string(),
            ));
        };
        if !workflow_ids.contains(active_workflow_id) {
            return Err(ApplicationError::Validation(format!(
                "active_workflow_id '{}' does not reference an indexed workflow",
                active_workflow_id
            )));
        }
    }

    Ok(())
}

fn validate_workflow_relative_path(field: &str, value: &str) -> Result<(), ApplicationError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(ApplicationError::Validation(format!(
            "{field} cannot be empty"
        )));
    }

    let path = Path::new(value);
    if path.is_absolute() {
        return Err(ApplicationError::Validation(format!(
            "{field} must use relative paths"
        )));
    }
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::Prefix(_) | Component::RootDir
        )
    }) {
        return Err(ApplicationError::Validation(format!(
            "{field} must not contain parent traversal"
        )));
    }

    Ok(())
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
        ApplicationError, GatewayValidationInput, HeaderRuleValidationInput, ModelValidationInput,
        ProviderValidationInput, RouteValidationInput, RuleScopeInput, WorkflowValidationInput,
        validate_gateway_basics,
    };

    fn valid_input() -> GatewayValidationInput {
        GatewayValidationInput {
            workflows_dir: Some("workflows".to_string()),
            active_workflow_id: Some("default".to_string()),
            providers: vec![ProviderValidationInput {
                id: "kimi".to_string(),
                base_url: "https://api.kimi.com".to_string(),
            }],
            models: vec![ModelValidationInput {
                id: "kimi-k2".to_string(),
                provider_id: "kimi".to_string(),
            }],
            routes: vec![RouteValidationInput {
                id: "chat".to_string(),
                matcher: "method == \"POST\"".to_string(),
                provider_id: "kimi".to_string(),
                model_id: Some("kimi-k2".to_string()),
            }],
            header_rules: vec![HeaderRuleValidationInput {
                id: "global".to_string(),
                scope: RuleScopeInput::Global,
                target_id: None,
                actions_len: 1,
            }],
            workflows: vec![WorkflowValidationInput {
                id: "default".to_string(),
                file: "default.toml".to_string(),
            }],
        }
    }

    #[test]
    fn validates_gateway_basics_successfully() {
        validate_gateway_basics(&valid_input()).expect("validation should pass");
    }

    #[test]
    fn rejects_duplicate_workflow_files() {
        let mut input = valid_input();
        input.workflows.push(WorkflowValidationInput {
            id: "fallback".to_string(),
            file: "default.toml".to_string(),
        });
        let error = validate_gateway_basics(&input).expect_err("duplicate file should fail");
        assert!(matches!(error, ApplicationError::Validation(_)));
    }
}
