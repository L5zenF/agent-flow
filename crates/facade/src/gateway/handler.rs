use std::sync::Arc;

use axum::body::to_bytes;
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use infrastructure::upstream_http::{build_upstream_url, forward_request};
use reqwest::Client;
use tokio::sync::RwLock;
use tracing::info;

use crate::config::RuntimeState;
use crate::gateway::runtime::{GatewayGraphExecutor, WASMTIME_PLUGIN_RUNTIME};
use crate::gateway_execution::{format_header_names, resolve_request};
use infrastructure::plugin_registry::PluginRegistry;

#[derive(Clone)]
pub struct GatewayState {
    pub client: Client,
    pub runtime_state: Arc<RwLock<RuntimeState>>,
    pub plugin_registry: Arc<PluginRegistry>,
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

    let runtime_state = state.runtime_state.read().await;
    let executor = GatewayGraphExecutor {
        runtime: &WASMTIME_PLUGIN_RUNTIME,
    };
    let resolution = match resolve_request(
        &runtime_state.config,
        &runtime_state.workflow_set,
        state.plugin_registry.as_ref(),
        &executor,
        &method,
        &uri,
        &headers,
    ) {
        Ok(Some(value)) => value,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                "no route matched request".to_string(),
            )
                .into_response();
        }
        Err(error) => return (StatusCode::BAD_REQUEST, error).into_response(),
    };

    let upstream_url =
        match build_upstream_url(&resolution.provider.base_url, &resolution.path, uri.query()) {
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
