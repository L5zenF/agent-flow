use std::collections::HashMap;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleScope {
    Global,
    Provider,
    Model,
    Route,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeaderAction {
    Set { name: String, value: String },
    Remove { name: String },
    Copy { from: String, to: String },
    SetIfAbsent { name: String, value: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderRule {
    pub id: String,
    pub enabled: bool,
    pub scope: RuleScope,
    pub target_id: Option<String>,
    pub when: Option<String>,
    pub actions: Vec<HeaderAction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderPolicy {
    rules: Vec<HeaderRule>,
}

impl HeaderPolicy {
    pub fn new(rules: Vec<HeaderRule>) -> Self {
        Self { rules }
    }

    pub fn resolve(
        &self,
        provider_defaults: &[(String, String)],
        request: &HeaderPolicyRequest<'_>,
    ) -> Result<Vec<(String, String)>, PolicyError> {
        let mut resolved = HashMap::<String, String>::new();
        for (name, value) in provider_defaults {
            resolved.insert(name.to_ascii_lowercase(), value.clone());
        }

        for rule in self.ordered_rules(request) {
            if !rule.enabled {
                continue;
            }
            if let Some(condition) = rule.when.as_deref()
                && !evaluate_expression(condition, request)?
            {
                continue;
            }
            apply_actions(&mut resolved, &rule.actions, request)?;
        }

        Ok(resolved.into_iter().collect())
    }

    fn ordered_rules<'a>(&'a self, request: &HeaderPolicyRequest<'_>) -> Vec<&'a HeaderRule> {
        let mut global = Vec::new();
        let mut provider = Vec::new();
        let mut model = Vec::new();
        let mut route = Vec::new();

        for rule in &self.rules {
            match rule.scope {
                RuleScope::Global => global.push(rule),
                RuleScope::Provider
                    if request
                        .provider_id
                        .map(|id| rule.target_id.as_deref() == Some(id))
                        .unwrap_or(false) =>
                {
                    provider.push(rule)
                }
                RuleScope::Model
                    if request
                        .model_id
                        .map(|id| rule.target_id.as_deref() == Some(id))
                        .unwrap_or(false) =>
                {
                    model.push(rule)
                }
                RuleScope::Route
                    if request
                        .route_id
                        .map(|id| rule.target_id.as_deref() == Some(id))
                        .unwrap_or(false) =>
                {
                    route.push(rule)
                }
                _ => {}
            }
        }

        global
            .into_iter()
            .chain(provider)
            .chain(model)
            .chain(route)
            .collect()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HeaderPolicyRequest<'a> {
    pub method: &'a str,
    pub path: &'a str,
    pub headers: &'a HashMap<String, String>,
    pub context: &'a HashMap<String, String>,
    pub provider_id: Option<&'a str>,
    pub provider_name: Option<&'a str>,
    pub model_id: Option<&'a str>,
    pub route_id: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyError {
    InvalidExpression(String),
    InvalidTemplate(String),
    UnsupportedValueSource(String),
    MissingRequestHeader(String),
    MissingContextValue(String),
    MissingEnvironmentValue(String),
}

impl Display for PolicyError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidExpression(message) => formatter.write_str(message),
            Self::InvalidTemplate(message) => formatter.write_str(message),
            Self::UnsupportedValueSource(source) => {
                write!(formatter, "unsupported value source '{source}'")
            }
            Self::MissingRequestHeader(key) => {
                write!(formatter, "request header '{key}' is unavailable")
            }
            Self::MissingContextValue(key) => {
                write!(formatter, "context value '{key}' is unavailable")
            }
            Self::MissingEnvironmentValue(key) => {
                write!(formatter, "environment variable '{key}' is not set")
            }
        }
    }
}

impl Error for PolicyError {}

fn apply_actions(
    resolved: &mut HashMap<String, String>,
    actions: &[HeaderAction],
    request: &HeaderPolicyRequest<'_>,
) -> Result<(), PolicyError> {
    for action in actions {
        match action {
            HeaderAction::Set { name, value } => {
                resolved.insert(name.to_ascii_lowercase(), render_template(value, request)?);
            }
            HeaderAction::Remove { name } => {
                resolved.remove(&name.to_ascii_lowercase());
            }
            HeaderAction::Copy { from, to } => {
                let source = request
                    .headers
                    .get(&from.to_ascii_lowercase())
                    .ok_or_else(|| PolicyError::MissingRequestHeader(from.clone()))?;
                resolved.insert(to.to_ascii_lowercase(), source.clone());
            }
            HeaderAction::SetIfAbsent { name, value } => {
                resolved
                    .entry(name.to_ascii_lowercase())
                    .or_insert(render_template(value, request)?);
            }
        }
    }
    Ok(())
}

pub fn evaluate_expression(
    expression: &str,
    request: &HeaderPolicyRequest<'_>,
) -> Result<bool, PolicyError> {
    evaluate_or(expression.trim(), request)
}

fn evaluate_or(expression: &str, request: &HeaderPolicyRequest<'_>) -> Result<bool, PolicyError> {
    let parts = split_top_level(expression, "||");
    let mut result = false;
    for part in parts {
        result = result || evaluate_and(part, request)?;
    }
    Ok(result)
}

fn evaluate_and(expression: &str, request: &HeaderPolicyRequest<'_>) -> Result<bool, PolicyError> {
    let parts = split_top_level(expression, "&&");
    let mut result = true;
    for part in parts {
        result = result && evaluate_atom(part.trim(), request)?;
    }
    Ok(result)
}

fn evaluate_atom(expression: &str, request: &HeaderPolicyRequest<'_>) -> Result<bool, PolicyError> {
    let expression = expression.trim();
    if expression.is_empty() {
        return Err(PolicyError::InvalidExpression(
            "empty expression".to_string(),
        ));
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
            return Ok(resolve_value(left.trim(), request)?
                .starts_with(&parse_string_literal(raw.trim())?));
        }
        if let Some((left, raw)) = value.split_once(".contains(") {
            return Ok(
                resolve_value(left.trim(), request)?.contains(&parse_string_literal(raw.trim())?)
            );
        }
    }

    Err(PolicyError::InvalidExpression(format!(
        "unsupported expression '{expression}'"
    )))
}

fn resolve_value(source: &str, request: &HeaderPolicyRequest<'_>) -> Result<String, PolicyError> {
    match source {
        "method" => Ok(request.method.to_string()),
        "path" => Ok(request.path.to_string()),
        "provider.id" => request
            .provider_id
            .map(str::to_string)
            .ok_or_else(|| PolicyError::UnsupportedValueSource(source.to_string())),
        "provider.name" => request
            .provider_name
            .map(str::to_string)
            .ok_or_else(|| PolicyError::UnsupportedValueSource(source.to_string())),
        "route.id" => request
            .route_id
            .map(str::to_string)
            .ok_or_else(|| PolicyError::UnsupportedValueSource(source.to_string())),
        "model.id" => request
            .model_id
            .map(str::to_string)
            .ok_or_else(|| PolicyError::UnsupportedValueSource(source.to_string())),
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
        _ if source.starts_with("header[") => {
            let key = parse_header_lookup(source)?;
            Ok(request
                .headers
                .get(&key.to_ascii_lowercase())
                .cloned()
                .unwrap_or_default())
        }
        _ => Err(PolicyError::UnsupportedValueSource(source.to_string())),
    }
}

fn parse_header_lookup(source: &str) -> Result<String, PolicyError> {
    let raw = source
        .strip_prefix("header[")
        .and_then(|value| value.strip_suffix(']'))
        .ok_or_else(|| {
            PolicyError::InvalidExpression(format!("invalid header lookup '{source}'"))
        })?;
    parse_string_literal(raw)
}

fn parse_string_literal(value: &str) -> Result<String, PolicyError> {
    let trimmed = value.trim();
    if (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
    {
        return Ok(trimmed[1..trimmed.len() - 1].to_string());
    }
    Err(PolicyError::InvalidExpression(format!(
        "expected string literal, got '{trimmed}'"
    )))
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

pub fn render_template(
    template: &str,
    request: &HeaderPolicyRequest<'_>,
) -> Result<String, PolicyError> {
    let mut output = template.to_string();

    replace_optional_token(&mut output, "${provider.id}", request.provider_id)?;
    replace_optional_token(&mut output, "${provider.name}", request.provider_name)?;
    replace_optional_token(&mut output, "${route.id}", request.route_id)?;
    replace_optional_token(&mut output, "${model.id}", request.model_id)?;

    while let Some(start) = output.find("${request.header.") {
        let Some(end) = output[start..].find('}') else {
            return Err(PolicyError::InvalidTemplate(
                "unterminated request.header template".to_string(),
            ));
        };
        let full = &output[start..start + end + 1];
        let key = full
            .strip_prefix("${request.header.")
            .and_then(|value| value.strip_suffix('}'))
            .ok_or_else(|| {
                PolicyError::InvalidTemplate(format!("invalid template variable '{full}'"))
            })?;
        let value = request
            .headers
            .get(&key.to_ascii_lowercase())
            .ok_or_else(|| PolicyError::MissingRequestHeader(key.to_string()))?;
        output = output.replacen(full, value, 1);
    }

    while let Some(start) = output.find("${ctx.") {
        let Some(end) = output[start..].find('}') else {
            return Err(PolicyError::InvalidTemplate(
                "unterminated ctx template".to_string(),
            ));
        };
        let full = &output[start..start + end + 1];
        let key = full
            .strip_prefix("${ctx.")
            .and_then(|value| value.strip_suffix('}'))
            .ok_or_else(|| {
                PolicyError::InvalidTemplate(format!("invalid template variable '{full}'"))
            })?;
        let value = request
            .context
            .get(key)
            .ok_or_else(|| PolicyError::MissingContextValue(key.to_string()))?;
        output = output.replacen(full, value, 1);
    }

    while let Some(start) = output.find("${context.") {
        let Some(end) = output[start..].find('}') else {
            return Err(PolicyError::InvalidTemplate(
                "unterminated context template".to_string(),
            ));
        };
        let full = &output[start..start + end + 1];
        let key = full
            .strip_prefix("${context.")
            .and_then(|value| value.strip_suffix('}'))
            .ok_or_else(|| {
                PolicyError::InvalidTemplate(format!("invalid template variable '{full}'"))
            })?;
        let value = request
            .context
            .get(key)
            .ok_or_else(|| PolicyError::MissingContextValue(key.to_string()))?;
        output = output.replacen(full, value, 1);
    }

    while let Some(start) = output.find("${env.") {
        let Some(end) = output[start..].find('}') else {
            return Err(PolicyError::InvalidTemplate(
                "unterminated env template".to_string(),
            ));
        };
        let full = &output[start..start + end + 1];
        let key = full
            .strip_prefix("${env.")
            .and_then(|value| value.strip_suffix('}'))
            .ok_or_else(|| {
                PolicyError::InvalidTemplate(format!("invalid template variable '{full}'"))
            })?;
        let value = std::env::var(key)
            .map_err(|_| PolicyError::MissingEnvironmentValue(key.to_string()))?;
        output = output.replacen(full, &value, 1);
    }

    Ok(output)
}

fn replace_optional_token(
    output: &mut String,
    token: &str,
    value: Option<&str>,
) -> Result<(), PolicyError> {
    if output.contains(token) {
        let value = value.ok_or_else(|| {
            PolicyError::InvalidTemplate(format!("template variable '{}' is unavailable", token))
        })?;
        *output = output.replace(token, value);
    }
    Ok(())
}
