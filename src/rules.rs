use std::collections::HashMap;

use axum::http::{HeaderMap, HeaderName, HeaderValue};

use crate::config::{GatewayConfig, HeaderActionConfig, HeaderRuleConfig, HeaderValueConfig, ProviderConfig, RouteConfig, RuleScope};
use crate::crypto::decrypt_header_value;

#[derive(Clone)]
pub struct RequestContext<'a> {
    pub method: &'a str,
    pub path: &'a str,
    pub headers: &'a HeaderMap,
    pub context: &'a HashMap<String, String>,
    pub provider: Option<&'a ProviderConfig>,
    pub model: Option<&'a crate::config::ModelConfig>,
    pub route: Option<&'a RouteConfig>,
}

pub fn build_header_map(
    config: &GatewayConfig,
    request: &RequestContext<'_>,
) -> Result<Vec<(HeaderName, HeaderValue)>, String> {
    let mut resolved = HashMap::<String, String>::new();
    let provider = request
        .provider
        .ok_or_else(|| "provider is unavailable for header resolution".to_string())?;

    for header in &provider.default_headers {
        let value = resolve_provider_header(header, config.default_secret_env.as_deref())?;
        resolved.insert(header.name.to_ascii_lowercase(), value);
    }

    for rule in ordered_rules(config, request) {
        if !rule.enabled {
            continue;
        }
        if let Some(condition) = rule.when.as_deref() {
            if !evaluate_expression(condition, request)? {
                continue;
            }
        }

        apply_actions(&mut resolved, &rule.actions, request)?;
    }

    resolved
        .into_iter()
        .map(|(name, value)| {
            let header_name = HeaderName::try_from(name).map_err(|error| error.to_string())?;
            let header_value = HeaderValue::from_str(&value).map_err(|error| error.to_string())?;
            Ok((header_name, header_value))
        })
        .collect()
}

fn ordered_rules<'a>(
    config: &'a GatewayConfig,
    request: &RequestContext<'_>,
) -> Vec<&'a HeaderRuleConfig> {
    let mut global = Vec::new();
    let mut provider = Vec::new();
    let mut model = Vec::new();
    let mut route = Vec::new();

    for rule in &config.header_rules {
        match rule.scope {
            RuleScope::Global => global.push(rule),
            RuleScope::Provider
                if request
                    .provider
                    .map(|provider| rule.target_id.as_deref() == Some(provider.id.as_str()))
                    .unwrap_or(false) =>
            {
                provider.push(rule)
            }
            RuleScope::Model
                if request
                    .model
                    .map(|model| rule.target_id.as_deref() == Some(model.id.as_str()))
                    .unwrap_or(false) =>
            {
                model.push(rule)
            }
            RuleScope::Route
                if request
                    .route
                    .map(|route| rule.target_id.as_deref() == Some(route.id.as_str()))
                    .unwrap_or(false) =>
            {
                route.push(rule)
            }
            _ => {}
        }
    }

    global.into_iter().chain(provider).chain(model).chain(route).collect()
}

fn resolve_provider_header(
    header: &crate::config::HeaderConfig,
    default_secret_env: Option<&str>,
) -> Result<String, String> {
    match &header.value {
        HeaderValueConfig::Plain { value } => Ok(value.clone()),
        HeaderValueConfig::Encrypted {
            value,
            encrypted: true,
            secret_env,
        } => decrypt_header_value(
            value,
            secret_env
                .as_deref()
                .or(default_secret_env)
                .ok_or_else(|| format!("header '{}' is encrypted but missing secret_env", header.name))?,
        ),
        HeaderValueConfig::Encrypted { value, .. } => Ok(value.clone()),
    }
}

fn apply_actions(
    resolved: &mut HashMap<String, String>,
    actions: &[HeaderActionConfig],
    request: &RequestContext<'_>,
) -> Result<(), String> {
    for action in actions {
        match action {
            HeaderActionConfig::Set { name, value } => {
                resolved.insert(name.to_ascii_lowercase(), render_template(value, request)?);
            }
            HeaderActionConfig::Remove { name } => {
                resolved.remove(&name.to_ascii_lowercase());
            }
            HeaderActionConfig::Copy { from, to } => {
                let source = request
                    .headers
                    .get(from)
                    .and_then(|value| value.to_str().ok())
                    .ok_or_else(|| format!("header '{from}' is unavailable for copy action"))?;
                resolved.insert(to.to_ascii_lowercase(), source.to_string());
            }
            HeaderActionConfig::SetIfAbsent { name, value } => {
                resolved
                    .entry(name.to_ascii_lowercase())
                    .or_insert(render_template(value, request)?);
            }
        }
    }

    Ok(())
}

pub fn evaluate_expression(expression: &str, request: &RequestContext<'_>) -> Result<bool, String> {
    evaluate_or(expression.trim(), request)
}

fn evaluate_or(expression: &str, request: &RequestContext<'_>) -> Result<bool, String> {
    let parts = split_top_level(expression, "||");
    let mut result = false;
    for part in parts {
        result = result || evaluate_and(part, request)?;
    }
    Ok(result)
}

fn evaluate_and(expression: &str, request: &RequestContext<'_>) -> Result<bool, String> {
    let parts = split_top_level(expression, "&&");
    let mut result = true;
    for part in parts {
        result = result && evaluate_atom(part.trim(), request)?;
    }
    Ok(result)
}

fn evaluate_atom(expression: &str, request: &RequestContext<'_>) -> Result<bool, String> {
    let expression = expression.trim();
    if expression.is_empty() {
        return Err("empty expression".to_string());
    }
    if let Some(rest) = expression.strip_prefix('!') {
        return Ok(!evaluate_atom(rest, request)?);
    }
    if let Some((left, right)) = expression.split_once("==") {
        return Ok(resolve_value(left.trim(), request)? == parse_string_literal(right.trim())?);
    }
    if let Some((left, right)) = expression.split_once("!=") {
        return Ok(resolve_value(left.trim(), request)? != parse_string_literal(right.trim())?);
    }
    if let Some(value) = expression.strip_suffix(')') {
        if let Some((left, raw)) = value.split_once(".startsWith(") {
            return Ok(resolve_value(left.trim(), request)?.starts_with(&parse_string_literal(raw.trim())?));
        }
        if let Some((left, raw)) = value.split_once(".contains(") {
            return Ok(resolve_value(left.trim(), request)?.contains(&parse_string_literal(raw.trim())?));
        }
    }

    Err(format!("unsupported expression '{expression}'"))
}

fn resolve_value(source: &str, request: &RequestContext<'_>) -> Result<String, String> {
    match source {
        "method" => Ok(request.method.to_string()),
        "path" => Ok(request.path.to_string()),
        "provider.id" => request
            .provider
            .map(|provider| provider.id.clone())
            .ok_or_else(|| "provider.id is unavailable".to_string()),
        "provider.name" => request
            .provider
            .map(|provider| provider.name.clone())
            .ok_or_else(|| "provider.name is unavailable".to_string()),
        _ if source.starts_with("ctx.") => Ok(request
            .context
            .get(source.trim_start_matches("ctx."))
            .cloned()
            .unwrap_or_default()),
        _ if source.starts_with("context.") => Ok(request
            .context
            .get(source.trim_start_matches("context."))
            .cloned()
            .unwrap_or_default()),
        "route.id" => request
            .route
            .map(|route| route.id.clone())
            .ok_or_else(|| "route.id is unavailable".to_string()),
        "model.id" => request
            .model
            .map(|model| model.id.clone())
            .ok_or_else(|| "model.id is unavailable".to_string()),
        _ if source.starts_with("header[") => {
            let key = parse_header_lookup(source)?;
            Ok(request
                .headers
                .get(&key)
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default()
                .to_string())
        }
        _ => Err(format!("unsupported value source '{source}'")),
    }
}

fn parse_header_lookup(source: &str) -> Result<String, String> {
    let raw = source
        .strip_prefix("header[")
        .and_then(|value| value.strip_suffix(']'))
        .ok_or_else(|| format!("invalid header lookup '{source}'"))?;
    parse_string_literal(raw)
}

fn parse_string_literal(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
    {
        return Ok(trimmed[1..trimmed.len() - 1].to_string());
    }
    Err(format!("expected string literal, got '{trimmed}'"))
}

fn split_top_level<'a>(expression: &'a str, token: &str) -> Vec<&'a str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut index = 0;
    let mut in_quote = false;
    let chars: Vec<char> = expression.chars().collect();

    while index + token.len() <= chars.len() {
        let current = chars[index];
        if current == '"' || current == '\'' {
            in_quote = !in_quote;
            index += 1;
            continue;
        }
        if !in_quote && expression[index..].starts_with(token) {
            parts.push(expression[start..index].trim());
            index += token.len();
            start = index;
            continue;
        }
        index += 1;
    }

    parts.push(expression[start..].trim());
    parts
}

pub fn render_template(template: &str, request: &RequestContext<'_>) -> Result<String, String> {
    let mut output = template.to_string();

    replace_optional_token(&mut output, "${provider.id}", request.provider.map(|provider| provider.id.as_str()))?;
    replace_optional_token(
        &mut output,
        "${provider.name}",
        request.provider.map(|provider| provider.name.as_str()),
    )?;
    replace_optional_token(
        &mut output,
        "${route.id}",
        request.route.map(|route| route.id.as_str()),
    )?;
    replace_optional_token(
        &mut output,
        "${model.id}",
        request.model.map(|model| model.id.as_str()),
    )?;

    while let Some(start) = output.find("${request.header.") {
        let Some(end) = output[start..].find('}') else {
            return Err("unterminated request.header template".to_string());
        };
        let full = &output[start..start + end + 1];
        let key = full
            .strip_prefix("${request.header.")
            .and_then(|value| value.strip_suffix('}'))
            .ok_or_else(|| format!("invalid template variable '{full}'"))?;
        let value = request
            .headers
            .get(key)
            .and_then(|header| header.to_str().ok())
            .ok_or_else(|| format!("request header '{key}' is unavailable"))?;
        output = output.replacen(full, value, 1);
    }

    while let Some(start) = output.find("${ctx.") {
        let Some(end) = output[start..].find('}') else {
            return Err("unterminated ctx template".to_string());
        };
        let full = &output[start..start + end + 1];
        let key = full
            .strip_prefix("${ctx.")
            .and_then(|value| value.strip_suffix('}'))
            .ok_or_else(|| format!("invalid template variable '{full}'"))?;
        let value = request
            .context
            .get(key)
            .ok_or_else(|| format!("ctx value '{key}' is unavailable"))?;
        output = output.replacen(full, value, 1);
    }

    while let Some(start) = output.find("${context.") {
        let Some(end) = output[start..].find('}') else {
            return Err("unterminated context template".to_string());
        };
        let full = &output[start..start + end + 1];
        let key = full
            .strip_prefix("${context.")
            .and_then(|value| value.strip_suffix('}'))
            .ok_or_else(|| format!("invalid template variable '{full}'"))?;
        let value = request
            .context
            .get(key)
            .ok_or_else(|| format!("context value '{key}' is unavailable"))?;
        output = output.replacen(full, value, 1);
    }

    while let Some(start) = output.find("${env.") {
        let Some(end) = output[start..].find('}') else {
            return Err("unterminated env template".to_string());
        };
        let full = &output[start..start + end + 1];
        let key = full
            .strip_prefix("${env.")
            .and_then(|value| value.strip_suffix('}'))
            .ok_or_else(|| format!("invalid template variable '{full}'"))?;
        let value = std::env::var(key)
            .map_err(|_| format!("environment variable '{key}' is not set"))?;
        output = output.replacen(full, &value, 1);
    }

    Ok(output)
}

fn replace_optional_token(
    output: &mut String,
    token: &str,
    value: Option<&str>,
) -> Result<(), String> {
    if output.contains(token) {
        let value = value.ok_or_else(|| format!("template variable '{}' is unavailable", token))?;
        *output = output.replace(token, value);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use axum::http::{HeaderMap, HeaderValue};

    use crate::config::{HeaderActionConfig, HeaderRuleConfig, ModelConfig, ProviderConfig, RouteConfig, RuleScope};

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

        assert!(evaluate_expression("method == \"POST\" && path.startsWith(\"/v1/\")", &request)
            .expect("expression should evaluate"));
        assert!(evaluate_expression("header[\"x-target\"] == \"kimi\"", &request)
            .expect("header equality should evaluate"));
        let mut context = HashMap::new();
        context.insert("intent".to_string(), "code".to_string());
        let request_with_context = RequestContext {
            context: &context,
            ..request
        };
        assert!(evaluate_expression("ctx.intent == \"code\"", &request_with_context)
            .expect("context equality should evaluate"));
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

        let rendered =
            render_template("${provider.id}:${model.id}:${route.id}", &request).expect("template should render");
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
