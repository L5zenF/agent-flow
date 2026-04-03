use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::extract::{Request, State};
use axum::http::header::{CONNECTION, HOST};
use axum::http::{HeaderMap, HeaderName, HeaderValue, Method, Response, StatusCode, Uri};
use axum::response::IntoResponse;
use reqwest::Client;
use tokio::sync::RwLock;
use tracing::{error, info};
use url::Url;

use crate::config::{
    ConditionMode, GatewayConfig, HeaderValueConfig, ModelConfig, ProviderConfig, RouteConfig,
    RuleGraphConfig, RuleGraphNodeType,
};
use crate::crypto::decrypt_header_value;
use crate::rules::{build_header_map, evaluate_expression, render_template, RequestContext};

#[derive(Clone)]
pub struct GatewayState {
    pub client: Client,
    pub config: Arc<RwLock<GatewayConfig>>,
}

pub async fn proxy_request(
    State(state): State<GatewayState>,
    request: Request,
) -> impl IntoResponse {
    let method = request.method().clone();
    let headers = request.headers().clone();
    let uri = request.uri().clone();
    let body = match to_bytes(request.into_body(), usize::MAX).await {
        Ok(body) => body,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                format!("failed to read request body: {error}"),
            )
                .into_response();
        }
    };

    let config = state.config.read().await;
    let resolution = match resolve_request(&config, &method, &uri, &headers) {
        Ok(Some(value)) => value,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, "no route matched request".to_string()).into_response()
        }
        Err(error) => return (StatusCode::BAD_REQUEST, error).into_response(),
    };

    let upstream_url = match build_upstream_url(&resolution.provider.base_url, &resolution.path, uri.query()) {
        Ok(url) => url,
        Err(error) => {
            return (
                StatusCode::BAD_GATEWAY,
                format!(
                    "invalid upstream url for provider '{}': {error}",
                    resolution.provider.id
                ),
            )
                .into_response();
        }
    };

    info!(
        route_id = %resolution.route.map(|item| item.id.as_str()).unwrap_or("<rule-graph>"),
        provider_id = %resolution.provider.id,
        model_id = %resolution.model.map(|item| item.id.as_str()).unwrap_or("<none>"),
        method = %method,
        upstream = %upstream_url,
        injected_headers = %format_header_names(&resolution.extra_headers),
        "forwarding request"
    );

    match forward_request(
        &state.client,
        method,
        headers.clone(),
        uri,
        upstream_url,
        body,
        &resolution.extra_headers,
    )
    .await
    {
        Ok(response) => response.into_response(),
        Err((status, message)) => (status, message).into_response(),
    }
}

struct RequestResolution<'a> {
    provider: &'a ProviderConfig,
    model: Option<&'a ModelConfig>,
    path: String,
    extra_headers: Vec<(HeaderName, HeaderValue)>,
    route: Option<&'a RouteConfig>,
}

fn resolve_request<'a>(
    config: &'a GatewayConfig,
    method: &Method,
    uri: &Uri,
    headers: &'a HeaderMap,
) -> Result<Option<RequestResolution<'a>>, String> {
    if let Some(graph) = &config.rule_graph {
        if !graph.nodes.is_empty() {
            return execute_rule_graph(config, graph, method, uri, headers).map(Some);
        }
    }

    let Some((route, provider, model)) = resolve_route(config, method, uri, headers) else {
        return Ok(None);
    };

    let request_context = RequestContext {
        method: method.as_str(),
        path: uri.path(),
        headers,
        provider: Some(provider),
        model,
        route: Some(route),
    };

    Ok(Some(RequestResolution {
        provider,
        model,
        path: route.path_rewrite.as_deref().unwrap_or(uri.path()).to_string(),
        extra_headers: build_header_map(config, &request_context)?,
        route: Some(route),
    }))
}

fn execute_rule_graph<'a>(
    config: &'a GatewayConfig,
    graph: &'a RuleGraphConfig,
    method: &Method,
    uri: &Uri,
    headers: &'a HeaderMap,
) -> Result<RequestResolution<'a>, String> {
    let node_map = graph
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect::<HashMap<_, _>>();
    let fallback_route = config.routes.first();
    let mut current_id = graph.start_node_id.as_str();
    let mut selected_provider: Option<&ProviderConfig> = None;
    let mut selected_model: Option<&ModelConfig> = None;
    let mut resolved_path = uri.path().to_string();
    let mut outgoing_headers = HashMap::<String, String>::new();
    let mut traversed = HashSet::<String>::new();
    let max_steps = graph.nodes.len().saturating_mul(3).max(1);

    for _ in 0..max_steps {
        let node = node_map
            .get(current_id)
            .copied()
            .ok_or_else(|| format!("rule_graph node '{}' does not exist", current_id))?;
        traversed.insert(current_id.to_string());

        let request_context = RequestContext {
            method: method.as_str(),
            path: &resolved_path,
            headers,
            provider: selected_provider,
            model: selected_model,
            route: fallback_route,
        };

        let next = match node.node_type {
            RuleGraphNodeType::Start => next_linear_edge(graph, current_id)?,
            RuleGraphNodeType::Condition => {
                let condition = node
                    .condition
                    .as_ref()
                    .ok_or_else(|| format!("rule_graph node '{}' missing condition config", node.id))?;
                let expression = match condition.mode {
                    ConditionMode::Expression => condition
                        .expression
                        .clone()
                        .ok_or_else(|| format!("rule_graph node '{}' missing expression", node.id))?,
                    ConditionMode::Builder => {
                        let builder = condition.builder.as_ref().ok_or_else(|| {
                            format!("rule_graph node '{}' missing builder config", node.id)
                        })?;
                        format!("{} {} \"{}\"", builder.field, builder.operator, builder.value)
                    }
                };
                let branch = if evaluate_expression(&expression, &request_context)? {
                    "true"
                } else {
                    "false"
                };
                next_condition_edge(graph, current_id, branch)?
            }
            RuleGraphNodeType::RouteProvider => {
                let provider_id = node
                    .route_provider
                    .as_ref()
                    .ok_or_else(|| format!("rule_graph node '{}' missing route_provider config", node.id))?
                    .provider_id
                    .as_str();
                selected_provider = Some(
                    config
                        .providers
                        .iter()
                        .find(|provider| provider.id == provider_id)
                        .ok_or_else(|| format!("rule_graph provider '{}' not found", provider_id))?,
                );

                if let Some(provider) = selected_provider {
                    for header in &provider.default_headers {
                        let value = resolve_provider_header_for_graph(header, config)?;
                        outgoing_headers.insert(header.name.to_ascii_lowercase(), value);
                    }
                }
                next_linear_edge(graph, current_id)?
            }
            RuleGraphNodeType::SelectModel => {
                let model_id = node
                    .select_model
                    .as_ref()
                    .ok_or_else(|| format!("rule_graph node '{}' missing select_model config", node.id))?
                    .model_id
                    .as_str();
                selected_model = Some(
                    config
                        .models
                        .iter()
                        .find(|model| model.id == model_id)
                        .ok_or_else(|| format!("rule_graph model '{}' not found", model_id))?,
                );
                next_linear_edge(graph, current_id)?
            }
            RuleGraphNodeType::RewritePath => {
                let node_config = node
                    .rewrite_path
                    .as_ref()
                    .ok_or_else(|| format!("rule_graph node '{}' missing rewrite_path config", node.id))?;
                resolved_path = render_template(&node_config.value, &request_context)?;
                next_linear_edge(graph, current_id)?
            }
            RuleGraphNodeType::SetHeader => {
                let node_config = node
                    .set_header
                    .as_ref()
                    .ok_or_else(|| format!("rule_graph node '{}' missing set_header config", node.id))?;
                outgoing_headers.insert(
                    node_config.name.to_ascii_lowercase(),
                    render_template(&node_config.value, &request_context)?,
                );
                next_linear_edge(graph, current_id)?
            }
            RuleGraphNodeType::RemoveHeader => {
                let node_config = node
                    .remove_header
                    .as_ref()
                    .ok_or_else(|| format!("rule_graph node '{}' missing remove_header config", node.id))?;
                outgoing_headers.remove(&node_config.name.to_ascii_lowercase());
                next_linear_edge(graph, current_id)?
            }
            RuleGraphNodeType::CopyHeader => {
                let node_config = node
                    .copy_header
                    .as_ref()
                    .ok_or_else(|| format!("rule_graph node '{}' missing copy_header config", node.id))?;
                let source = headers
                    .get(node_config.from.as_str())
                    .and_then(|value| value.to_str().ok())
                    .ok_or_else(|| {
                        format!("header '{}' is unavailable for graph copy action", node_config.from)
                    })?;
                outgoing_headers.insert(node_config.to.to_ascii_lowercase(), source.to_string());
                next_linear_edge(graph, current_id)?
            }
            RuleGraphNodeType::SetHeaderIfAbsent => {
                let node_config = node.set_header_if_absent.as_ref().ok_or_else(|| {
                    format!("rule_graph node '{}' missing set_header_if_absent config", node.id)
                })?;
                outgoing_headers
                    .entry(node_config.name.to_ascii_lowercase())
                    .or_insert(render_template(&node_config.value, &request_context)?);
                next_linear_edge(graph, current_id)?
            }
            RuleGraphNodeType::End => break,
        };

        current_id = match next {
            Some(next_id) => next_id,
            None => break,
        };
    }

    if traversed.len() >= max_steps {
        return Err("rule_graph exceeded maximum execution steps".to_string());
    }

    let provider = selected_provider.ok_or_else(|| "rule_graph did not select a provider".to_string())?;
    let mut extra_headers = Vec::new();
    for (name, value) in outgoing_headers {
        let header_name = HeaderName::try_from(name).map_err(|error| error.to_string())?;
        let header_value = HeaderValue::from_str(&value).map_err(|error| error.to_string())?;
        extra_headers.push((header_name, header_value));
    }

    Ok(RequestResolution {
        provider,
        model: selected_model,
        path: resolved_path,
        extra_headers,
        route: None,
    })
}

fn resolve_provider_header_for_graph(
    header: &crate::config::HeaderConfig,
    config: &GatewayConfig,
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
                .or(config.default_secret_env.as_deref())
                .ok_or_else(|| format!("header '{}' is encrypted but missing secret_env", header.name))?,
        ),
        HeaderValueConfig::Encrypted { value, .. } => Ok(value.clone()),
    }
}

fn next_linear_edge<'a>(graph: &'a RuleGraphConfig, node_id: &str) -> Result<Option<&'a str>, String> {
    let edges = graph
        .edges
        .iter()
        .filter(|edge| edge.source == node_id)
        .collect::<Vec<_>>();
    if edges.len() > 1 {
        return Err(format!(
            "rule_graph node '{}' has multiple outgoing edges but is not a condition node",
            node_id
        ));
    }
    Ok(edges.first().map(|edge| edge.target.as_str()))
}

fn next_condition_edge<'a>(
    graph: &'a RuleGraphConfig,
    node_id: &str,
    branch: &str,
) -> Result<Option<&'a str>, String> {
    Ok(graph
        .edges
        .iter()
        .find(|edge| edge.source == node_id && edge.source_handle.as_deref() == Some(branch))
        .map(|edge| edge.target.as_str()))
}

fn resolve_route<'a>(
    config: &'a GatewayConfig,
    method: &Method,
    uri: &Uri,
    headers: &'a HeaderMap,
) -> Option<(&'a RouteConfig, &'a ProviderConfig, Option<&'a ModelConfig>)> {
    let mut routes = config.routes.iter().filter(|route| route.enabled).collect::<Vec<_>>();
    routes.sort_by(|left, right| right.priority.cmp(&left.priority));

    for route in routes {
        let provider = config.providers.iter().find(|provider| provider.id == route.provider_id)?;
        let model = route
            .model_id
            .as_deref()
            .and_then(|model_id| config.models.iter().find(|model| model.id == model_id));
        let request_context = RequestContext {
            method: method.as_str(),
            path: uri.path(),
            headers,
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

async fn forward_request(
    client: &Client,
    method: Method,
    incoming_headers: HeaderMap,
    incoming_uri: Uri,
    upstream_url: Url,
    body: bytes::Bytes,
    extra_headers: &[(HeaderName, HeaderValue)],
) -> Result<Response<Body>, (StatusCode, String)> {
    let reqwest_method =
        reqwest::Method::from_bytes(method.as_str().as_bytes()).map_err(internal_error)?;
    let mut builder = client.request(reqwest_method, upstream_url);

    for (name, value) in incoming_headers.iter() {
        if should_skip_forward_header(name, extra_headers) {
            continue;
        }
        builder = builder.header(name, value);
    }

    for (name, value) in extra_headers {
        builder = builder.header(name, value);
    }

    let upstream_response = builder.body(body).send().await.map_err(bad_gateway_error)?;
    let status = upstream_response.status();
    let response_headers = upstream_response.headers().clone();
    let response_body = Body::from_stream(upstream_response.bytes_stream());

    let mut response = Response::builder().status(status);
    for (name, value) in response_headers.iter() {
        if name == CONNECTION {
            continue;
        }
        response = response.header(name, value);
    }

    response.body(response_body).map_err(|error| {
        error!(
            method = %method,
            uri = %incoming_uri,
            "failed to build response: {error}"
        );
        internal_error(error)
    })
}

fn build_upstream_url(base_url: &str, path: &str, query: Option<&str>) -> Result<Url, url::ParseError> {
    let mut url = Url::parse(base_url)?;
    url.set_path(path);
    url.set_query(query);
    Ok(url)
}

fn should_skip_forward_header(
    name: &HeaderName,
    extra_headers: &[(HeaderName, HeaderValue)],
) -> bool {
    name == HOST
        || name == CONNECTION
        || name.as_str().eq_ignore_ascii_case("x-target")
        || extra_headers.iter().any(|(extra_name, _)| extra_name == name)
}

fn format_header_names(headers: &[(HeaderName, HeaderValue)]) -> String {
    if headers.is_empty() {
        return "<none>".to_string();
    }
    headers
        .iter()
        .map(|(name, _)| name.as_str().to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

fn internal_error(error: impl std::fmt::Display) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}

fn bad_gateway_error(error: impl std::fmt::Display) -> (StatusCode, String) {
    (StatusCode::BAD_GATEWAY, error.to_string())
}
