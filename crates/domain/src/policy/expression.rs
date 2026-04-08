use super::{HeaderPolicyRequest, PolicyError};

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
