use std::collections::HashMap;

use application::{
    evaluate_policy_expression, render_policy_template, resolve_policy_headers, HeaderActionInput,
    HeaderRuleInput, HeaderValueInput, PolicyRequestFacts, ProviderDefaultHeaderInput,
    RuleScopeInput,
};
use axum::http::{HeaderMap, HeaderName, HeaderValue};
use infrastructure::crypto::decrypt_header_value;

use crate::config::{
    GatewayConfig, HeaderActionConfig, HeaderRuleConfig, HeaderValueConfig, ModelConfig,
    ProviderConfig, RouteConfig, RuleScope,
};

#[derive(Clone)]
pub struct RequestContext<'a> {
    pub method: &'a str,
    pub path: &'a str,
    pub headers: &'a HeaderMap,
    pub context: &'a HashMap<String, String>,
    pub provider: Option<&'a ProviderConfig>,
    pub model: Option<&'a ModelConfig>,
    pub route: Option<&'a RouteConfig>,
}

pub fn build_header_map(
    config: &GatewayConfig,
    request: &RequestContext<'_>,
) -> Result<Vec<(HeaderName, HeaderValue)>, String> {
    let provider = request
        .provider
        .ok_or_else(|| "provider is unavailable for header resolution".to_string())?;
    let provider_defaults = provider
        .default_headers
        .iter()
        .map(|header| ProviderDefaultHeaderInput {
            name: header.name.clone(),
            value: map_header_value(&header.value),
        })
        .collect::<Vec<_>>();
    let facts = to_policy_request_facts(request);
    let header_rules = map_header_rules(&config.header_rules);
    let resolved = resolve_policy_headers(
        &provider_defaults,
        config.default_secret_env.as_deref(),
        &header_rules,
        &facts,
        decrypt_header_value,
    )
    .map_err(|error| error.to_string())?;

    resolved
        .into_iter()
        .map(|(name, value)| {
            let header_name = HeaderName::try_from(name).map_err(|error| error.to_string())?;
            let header_value = HeaderValue::from_str(&value).map_err(|error| error.to_string())?;
            Ok((header_name, header_value))
        })
        .collect()
}

pub fn evaluate_expression(expression: &str, request: &RequestContext<'_>) -> Result<bool, String> {
    evaluate_policy_expression(expression, &to_policy_request_facts(request))
        .map_err(|error| error.to_string())
}

pub fn render_template(template: &str, request: &RequestContext<'_>) -> Result<String, String> {
    render_policy_template(template, &to_policy_request_facts(request))
        .map_err(|error| error.to_string())
}

fn to_policy_request_facts(request: &RequestContext<'_>) -> PolicyRequestFacts {
    PolicyRequestFacts {
        method: request.method.to_string(),
        path: request.path.to_string(),
        headers: normalize_headers(request.headers),
        context: request.context.clone(),
        provider_id: request.provider.map(|provider| provider.id.clone()),
        provider_name: request.provider.map(|provider| provider.name.clone()),
        model_id: request.model.map(|model| model.id.clone()),
        route_id: request.route.map(|route| route.id.clone()),
    }
}

fn map_header_rules(rules: &[HeaderRuleConfig]) -> Vec<HeaderRuleInput> {
    rules
        .iter()
        .map(|rule| HeaderRuleInput {
            id: rule.id.clone(),
            enabled: rule.enabled,
            scope: map_scope(rule.scope),
            target_id: rule.target_id.clone(),
            when: rule.when.clone(),
            actions: map_actions(&rule.actions),
        })
        .collect()
}

fn map_scope(scope: RuleScope) -> RuleScopeInput {
    match scope {
        RuleScope::Global => RuleScopeInput::Global,
        RuleScope::Provider => RuleScopeInput::Provider,
        RuleScope::Model => RuleScopeInput::Model,
        RuleScope::Route => RuleScopeInput::Route,
    }
}

fn map_actions(actions: &[HeaderActionConfig]) -> Vec<HeaderActionInput> {
    actions
        .iter()
        .map(|action| match action {
            HeaderActionConfig::Set { name, value } => HeaderActionInput::Set {
                name: name.clone(),
                value: value.clone(),
            },
            HeaderActionConfig::Remove { name } => HeaderActionInput::Remove { name: name.clone() },
            HeaderActionConfig::Copy { from, to } => HeaderActionInput::Copy {
                from: from.clone(),
                to: to.clone(),
            },
            HeaderActionConfig::SetIfAbsent { name, value } => HeaderActionInput::SetIfAbsent {
                name: name.clone(),
                value: value.clone(),
            },
        })
        .collect()
}

fn map_header_value(value: &HeaderValueConfig) -> HeaderValueInput {
    match value {
        HeaderValueConfig::Plain { value } => HeaderValueInput::Plain(value.clone()),
        HeaderValueConfig::Encrypted {
            value,
            encrypted,
            secret_env,
        } => HeaderValueInput::Encrypted {
            value: value.clone(),
            encrypted: *encrypted,
            secret_env: secret_env.clone(),
        },
    }
}

fn normalize_headers(headers: &HeaderMap) -> HashMap<String, String> {
    headers
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|value| (name.as_str().to_ascii_lowercase(), value.to_string()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use axum::http::{HeaderMap, HeaderValue};

    use crate::config::{
        HeaderActionConfig, HeaderRuleConfig, ModelConfig, ProviderConfig, RouteConfig, RuleScope,
    };

    use super::{evaluate_expression, render_template, RequestContext};

    #[test]
    fn evaluates_basic_expressions() {
        let mut headers = HeaderMap::new();
        headers.insert("x-target", HeaderValue::from_static("kimi"));
        let provider = ProviderConfig {
            id: "kimi".to_string(),
            name: "Kimi".to_string(),
            base_url: "https://api.kimi.com".to_string(),
            default_headers: Vec::new(),
        };
        let model = ModelConfig {
            id: "kimi-k2".to_string(),
            name: "Kimi K2".to_string(),
            provider_id: "kimi".to_string(),
            description: None,
        };
        let route = RouteConfig {
            id: "chat-default".to_string(),
            priority: 100,
            enabled: true,
            matcher: "path.startsWith(\"/v1/\")".to_string(),
            provider_id: "kimi".to_string(),
            model_id: Some("kimi-k2".to_string()),
            path_rewrite: None,
        };
        let empty_context = HashMap::new();
        let request = RequestContext {
            method: "POST",
            path: "/v1/chat/completions",
            headers: &headers,
            context: &empty_context,
            provider: Some(&provider),
            model: Some(&model),
            route: Some(&route),
        };

        assert!(
            evaluate_expression("method == \"POST\" && path.startsWith(\"/v1/\")", &request)
                .expect("expression should evaluate")
        );
        assert!(
            evaluate_expression("header[\"x-target\"] == \"kimi\"", &request)
                .expect("header equality should evaluate")
        );
        let mut context = HashMap::new();
        context.insert("intent".to_string(), "code".to_string());
        let request_with_context = RequestContext {
            context: &context,
            ..request
        };
        assert!(
            evaluate_expression("ctx.intent == \"code\"", &request_with_context)
                .expect("context equality should evaluate")
        );
    }

    #[test]
    fn renders_known_templates() {
        let headers = HeaderMap::new();
        let provider = ProviderConfig {
            id: "kimi".to_string(),
            name: "Kimi".to_string(),
            base_url: "https://api.kimi.com".to_string(),
            default_headers: Vec::new(),
        };
        let model = ModelConfig {
            id: "kimi-k2".to_string(),
            name: "Kimi K2".to_string(),
            provider_id: "kimi".to_string(),
            description: None,
        };
        let route = RouteConfig {
            id: "chat-default".to_string(),
            priority: 100,
            enabled: true,
            matcher: "method == \"POST\"".to_string(),
            provider_id: "kimi".to_string(),
            model_id: Some("kimi-k2".to_string()),
            path_rewrite: None,
        };
        let empty_context = HashMap::new();
        let request = RequestContext {
            method: "POST",
            path: "/v1/chat/completions",
            headers: &headers,
            context: &empty_context,
            provider: Some(&provider),
            model: Some(&model),
            route: Some(&route),
        };

        let rendered = render_template("${provider.id}:${model.id}:${route.id}", &request)
            .expect("template should render");
        assert_eq!(rendered, "kimi:kimi-k2:chat-default");
        let mut context = HashMap::new();
        context.insert("route_hint".to_string(), "kimi".to_string());
        let request_with_context = RequestContext {
            context: &context,
            ..request
        };
        assert_eq!(
            render_template("${ctx.route_hint}:${model.id}", &request_with_context)
                .expect("context template should render"),
            "kimi:kimi-k2"
        );
    }

    #[test]
    fn keeps_rule_structs_constructible() {
        let _rule = HeaderRuleConfig {
            id: "test".to_string(),
            enabled: true,
            scope: RuleScope::Global,
            target_id: None,
            when: None,
            actions: vec![HeaderActionConfig::Remove {
                name: "x-debug".to_string(),
            }],
        };
    }
}
