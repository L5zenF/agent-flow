use std::collections::HashMap;

use axum::http::{HeaderMap, HeaderName, HeaderValue, Method, Uri};

use crate::config::{GatewayConfig, ModelConfig, ProviderConfig, RouteConfig, RouterClauseConfig};
use crate::rules::{RequestContext, evaluate_expression};

use super::context::inject_runtime_context;

pub fn resolve_route<'a>(
    config: &'a GatewayConfig,
    method: &Method,
    uri: &Uri,
    headers: &'a HeaderMap,
) -> Option<(&'a RouteConfig, &'a ProviderConfig, Option<&'a ModelConfig>)> {
    let mut workflow_context = HashMap::new();
    inject_runtime_context(
        &mut workflow_context,
        method.as_str(),
        uri.path(),
        headers,
        None,
        None,
        None,
    );
    let mut routes = config
        .routes
        .iter()
        .filter(|route| route.enabled)
        .collect::<Vec<_>>();
    routes.sort_by(|left, right| right.priority.cmp(&left.priority));

    for route in routes {
        let provider = config
            .providers
            .iter()
            .find(|provider| provider.id == route.provider_id)?;
        let model = route
            .model_id
            .as_deref()
            .and_then(|model_id| config.models.iter().find(|model| model.id == model_id));
        let request_context = RequestContext {
            method: method.as_str(),
            path: uri.path(),
            headers,
            context: &workflow_context,
            provider: Some(provider),
            model,
            route: Some(route),
        };
        if evaluate_expression(&route.matcher, &request_context).ok()? {
            return Some((route, provider, model));
        }
    }

    None
}

pub fn evaluate_router_clause(
    clause: &RouterClauseConfig,
    request: &RequestContext<'_>,
) -> Result<bool, String> {
    let source = clause.source.trim();
    let operator = clause.operator.trim();
    let value = clause.value.trim().replace('"', "\\\"");
    let expression = match operator {
        "==" | "!=" => format!(r#"{source} {operator} "{value}""#),
        "startsWith" | "contains" => format!(r#"{source}.{operator}("{value}")"#),
        _ => return Err(format!("unsupported router operator '{}'", clause.operator)),
    };
    evaluate_expression(&expression, request)
}

pub fn format_header_names(headers: &[(HeaderName, HeaderValue)]) -> String {
    if headers.is_empty() {
        return "<none>".to_string();
    }
    headers
        .iter()
        .map(|(name, _)| name.as_str().to_string())
        .collect::<Vec<_>>()
        .join(", ")
}
