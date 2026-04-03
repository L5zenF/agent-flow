use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::extract::{Request, State};
use axum::http::header::{CONNECTION, HOST};
use axum::http::{HeaderMap, Method, Response, StatusCode, Uri};
use axum::response::IntoResponse;
use reqwest::Client;
use tokio::sync::RwLock;
use tracing::{error, info};
use url::Url;

use crate::config::{GatewayConfig, ModelConfig, ProviderConfig, RouteConfig};
use crate::rules::{build_header_map, evaluate_expression, RequestContext};

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
    let Some((route, provider, model)) = resolve_route(&config, &method, &uri, &headers) else {
        return (StatusCode::NOT_FOUND, "no route matched request".to_string()).into_response();
    };

    let path = route.path_rewrite.as_deref().unwrap_or(uri.path());
    let upstream_url = match build_upstream_url(&provider.base_url, path, uri.query()) {
        Ok(url) => url,
        Err(error) => {
            return (
                StatusCode::BAD_GATEWAY,
                format!("invalid upstream url for provider '{}': {error}", provider.id),
            )
                .into_response();
        }
    };

    let request_context = RequestContext {
        method: method.as_str(),
        path: uri.path(),
        headers: &headers,
        provider,
        model,
        route,
    };
    let extra_headers = match build_header_map(&config, &request_context) {
        Ok(headers) => headers,
        Err(error) => return (StatusCode::BAD_REQUEST, error).into_response(),
    };

    info!(
        route_id = %route.id,
        provider_id = %provider.id,
        model_id = %model.map(|item| item.id.as_str()).unwrap_or("<none>"),
        method = %method,
        upstream = %upstream_url,
        injected_headers = %format_header_names(&extra_headers),
        "forwarding request"
    );

    match forward_request(&state.client, method, headers, uri, upstream_url, body, &extra_headers).await {
        Ok(response) => response.into_response(),
        Err((status, message)) => (status, message).into_response(),
    }
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
            provider,
            model,
            route,
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
    extra_headers: &[(axum::http::HeaderName, axum::http::HeaderValue)],
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

fn build_upstream_url(
    base_url: &str,
    path: &str,
    query: Option<&str>,
) -> Result<Url, url::ParseError> {
    let mut url = Url::parse(base_url)?;
    url.set_path(path);
    url.set_query(query);
    Ok(url)
}

fn should_skip_forward_header(
    name: &axum::http::HeaderName,
    extra_headers: &[(axum::http::HeaderName, axum::http::HeaderValue)],
) -> bool {
    name == HOST
        || name == CONNECTION
        || name.as_str().eq_ignore_ascii_case("x-target")
        || extra_headers
            .iter()
            .any(|(extra_name, _)| extra_name == name)
}

fn format_header_names(headers: &[(axum::http::HeaderName, axum::http::HeaderValue)]) -> String {
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
