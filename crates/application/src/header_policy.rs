use std::collections::HashMap;

use domain::{
    HeaderAction as DomainHeaderAction, HeaderPolicy, HeaderPolicyRequest,
    HeaderRule as DomainHeaderRule, RuleScope as DomainRuleScope, evaluate_policy_expression,
    render_policy_template,
};

use crate::error::ApplicationError;

#[derive(Debug, Clone, Copy)]
pub enum RuleScopeInput {
    Global,
    Provider,
    Model,
    Route,
}

#[derive(Debug, Clone)]
pub enum HeaderActionInput {
    Set { name: String, value: String },
    Remove { name: String },
    Copy { from: String, to: String },
    SetIfAbsent { name: String, value: String },
}

#[derive(Debug, Clone)]
pub enum HeaderValueInput {
    Plain(String),
    Encrypted {
        value: String,
        encrypted: bool,
        secret_env: Option<String>,
    },
}

#[derive(Debug, Clone)]
pub struct ProviderDefaultHeaderInput {
    pub name: String,
    pub value: HeaderValueInput,
}

#[derive(Debug, Clone)]
pub struct HeaderRuleInput {
    pub id: String,
    pub enabled: bool,
    pub scope: RuleScopeInput,
    pub target_id: Option<String>,
    pub when: Option<String>,
    pub actions: Vec<HeaderActionInput>,
}

#[derive(Debug, Clone, Default)]
pub struct PolicyRequestFacts {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub context: HashMap<String, String>,
    pub provider_id: Option<String>,
    pub provider_name: Option<String>,
    pub model_id: Option<String>,
    pub route_id: Option<String>,
}

pub fn resolve_headers<F>(
    provider_defaults: &[ProviderDefaultHeaderInput],
    default_secret_env: Option<&str>,
    header_rules: &[HeaderRuleInput],
    request: &PolicyRequestFacts,
    decrypt_header: F,
) -> Result<Vec<(String, String)>, ApplicationError>
where
    F: Fn(&str, &str) -> Result<String, String>,
{
    let resolved_provider_defaults = provider_defaults
        .iter()
        .map(|header| {
            Ok((
                header.name.clone(),
                resolve_provider_header_value(
                    &header.name,
                    &header.value,
                    default_secret_env,
                    &decrypt_header,
                )?,
            ))
        })
        .collect::<Result<Vec<_>, ApplicationError>>()?;

    let rules = header_rules
        .iter()
        .map(|rule| DomainHeaderRule {
            id: rule.id.clone(),
            enabled: rule.enabled,
            scope: map_scope(rule.scope),
            target_id: rule.target_id.clone(),
            when: rule.when.clone(),
            actions: rule.actions.iter().map(map_action).collect(),
        })
        .collect::<Vec<_>>();
    let policy = HeaderPolicy::new(rules);
    policy
        .resolve(&resolved_provider_defaults, &to_domain_request(request))
        .map_err(|error| ApplicationError::Policy(error.to_string()))
}

pub fn evaluate_expression(
    expression: &str,
    request: &PolicyRequestFacts,
) -> Result<bool, ApplicationError> {
    evaluate_policy_expression(expression, &to_domain_request(request))
        .map_err(|error| ApplicationError::Policy(error.to_string()))
}

pub fn render_template(
    template: &str,
    request: &PolicyRequestFacts,
) -> Result<String, ApplicationError> {
    render_policy_template(template, &to_domain_request(request))
        .map_err(|error| ApplicationError::Policy(error.to_string()))
}

fn resolve_provider_header_value<F>(
    header_name: &str,
    header: &HeaderValueInput,
    default_secret_env: Option<&str>,
    decrypt_header: F,
) -> Result<String, ApplicationError>
where
    F: Fn(&str, &str) -> Result<String, String>,
{
    match header {
        HeaderValueInput::Plain(value) => Ok(value.clone()),
        HeaderValueInput::Encrypted {
            value,
            encrypted: true,
            secret_env,
        } => decrypt_header(
            value,
            secret_env
                .as_deref()
                .or(default_secret_env)
                .ok_or_else(|| {
                    ApplicationError::Policy(format!(
                        "header '{header_name}' is encrypted but missing secret_env"
                    ))
                })?,
        )
        .map_err(ApplicationError::Policy),
        HeaderValueInput::Encrypted { value, .. } => Ok(value.clone()),
    }
}

fn to_domain_request(request: &PolicyRequestFacts) -> HeaderPolicyRequest<'_> {
    HeaderPolicyRequest {
        method: &request.method,
        path: &request.path,
        headers: &request.headers,
        context: &request.context,
        provider_id: request.provider_id.as_deref(),
        provider_name: request.provider_name.as_deref(),
        model_id: request.model_id.as_deref(),
        route_id: request.route_id.as_deref(),
    }
}

fn map_scope(scope: RuleScopeInput) -> DomainRuleScope {
    match scope {
        RuleScopeInput::Global => DomainRuleScope::Global,
        RuleScopeInput::Provider => DomainRuleScope::Provider,
        RuleScopeInput::Model => DomainRuleScope::Model,
        RuleScopeInput::Route => DomainRuleScope::Route,
    }
}

fn map_action(action: &HeaderActionInput) -> DomainHeaderAction {
    match action {
        HeaderActionInput::Set { name, value } => DomainHeaderAction::Set {
            name: name.clone(),
            value: value.clone(),
        },
        HeaderActionInput::Remove { name } => DomainHeaderAction::Remove { name: name.clone() },
        HeaderActionInput::Copy { from, to } => DomainHeaderAction::Copy {
            from: from.clone(),
            to: to.clone(),
        },
        HeaderActionInput::SetIfAbsent { name, value } => DomainHeaderAction::SetIfAbsent {
            name: name.clone(),
            value: value.clone(),
        },
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::header_policy::{
        HeaderActionInput, HeaderRuleInput, HeaderValueInput, PolicyRequestFacts,
        ProviderDefaultHeaderInput, RuleScopeInput, evaluate_expression, resolve_headers,
    };

    #[test]
    fn resolves_headers_with_policy_actions() {
        let defaults = vec![ProviderDefaultHeaderInput {
            name: "authorization".to_string(),
            value: HeaderValueInput::Plain("Bearer test".to_string()),
        }];
        let rules = vec![HeaderRuleInput {
            id: "r1".to_string(),
            enabled: true,
            scope: RuleScopeInput::Global,
            target_id: None,
            when: Some("path.startsWith(\"/v1/\")".to_string()),
            actions: vec![HeaderActionInput::Set {
                name: "x-target".to_string(),
                value: "ok".to_string(),
            }],
        }];
        let facts = PolicyRequestFacts {
            method: "POST".to_string(),
            path: "/v1/chat/completions".to_string(),
            headers: HashMap::new(),
            context: HashMap::new(),
            provider_id: Some("kimi".to_string()),
            provider_name: Some("Kimi".to_string()),
            model_id: Some("kimi-k2".to_string()),
            route_id: None,
        };

        let resolved = resolve_headers(&defaults, None, &rules, &facts, |value, _secret_env| {
            Ok(value.to_string())
        })
        .expect("headers should resolve");
        assert!(
            resolved
                .iter()
                .any(|(name, value)| { name.eq_ignore_ascii_case("x-target") && value == "ok" })
        );
    }

    #[test]
    fn evaluates_expression_from_request_facts() {
        let mut context = HashMap::new();
        context.insert("intent".to_string(), "code".to_string());
        let facts = PolicyRequestFacts {
            method: "POST".to_string(),
            path: "/v1/chat/completions".to_string(),
            headers: HashMap::new(),
            context,
            provider_id: None,
            provider_name: None,
            model_id: None,
            route_id: None,
        };

        assert!(
            evaluate_expression("ctx.intent == \"code\"", &facts)
                .expect("expression should evaluate"),
        );
    }
}
