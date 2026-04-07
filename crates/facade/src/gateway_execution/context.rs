use std::collections::HashMap;

use axum::http::HeaderMap;
use infrastructure::crypto::decrypt_header_value;

use crate::config::{
    GatewayConfig, HeaderConfig, HeaderValueConfig, ModelConfig, ProviderConfig, RouteConfig,
    RuleGraphConfig,
};

pub(crate) const SELECTED_PROVIDER_CONTEXT_KEY: &str = "selection.provider_id";
pub(crate) const SELECTED_MODEL_CONTEXT_KEY: &str = "selection.model_id";

pub fn resolve_provider_header_for_graph(
    header: &HeaderConfig,
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
                .ok_or_else(|| {
                    format!(
                        "header '{}' is encrypted but missing secret_env",
                        header.name
                    )
                })?,
        ),
        HeaderValueConfig::Encrypted { value, .. } => Ok(value.clone()),
    }
}

pub fn next_linear_edge<'a>(
    graph: &'a RuleGraphConfig,
    node_id: &str,
) -> Result<Option<&'a str>, String> {
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

pub fn next_condition_edge<'a>(
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

pub fn sync_selected_targets_from_context<'a>(
    config: &'a GatewayConfig,
    workflow_context: &HashMap<String, String>,
    outgoing_headers: &mut HashMap<String, Vec<String>>,
    selected_provider: &mut Option<&'a ProviderConfig>,
    selected_model: &mut Option<&'a ModelConfig>,
    selected_provider_context_key: &str,
    selected_model_context_key: &str,
) -> Result<(), String> {
    let requested_provider_id = workflow_context
        .get(selected_provider_context_key)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty());
    let requested_model_id = workflow_context
        .get(selected_model_context_key)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty());

    let resolved_model = if let Some(model_id) = requested_model_id {
        Some(
            config
                .models
                .iter()
                .find(|model| model.id == model_id)
                .ok_or_else(|| format!("rule_graph model '{}' not found", model_id))?,
        )
    } else {
        *selected_model
    };

    let provider_id =
        requested_provider_id.or_else(|| resolved_model.map(|model| model.provider_id.as_str()));
    let resolved_provider = if let Some(provider_id) = provider_id {
        Some(
            config
                .providers
                .iter()
                .find(|provider| provider.id == provider_id)
                .ok_or_else(|| format!("rule_graph provider '{}' not found", provider_id))?,
        )
    } else {
        *selected_provider
    };

    if let (Some(provider), Some(model)) = (resolved_provider, resolved_model) {
        if model.provider_id != provider.id {
            return Err(format!(
                "rule_graph model '{}' does not belong to provider '{}'",
                model.id, provider.id
            ));
        }
    }

    if resolved_provider.map(|provider| provider.id.as_str())
        != selected_provider.map(|provider| provider.id.as_str())
    {
        if let Some(provider) = resolved_provider {
            for header in &provider.default_headers {
                let value = resolve_provider_header_for_graph(header, config)?;
                outgoing_headers.insert(header.name.to_ascii_lowercase(), vec![value]);
            }
        }
    }

    *selected_provider = resolved_provider;
    *selected_model = match (resolved_provider, resolved_model) {
        (Some(provider), Some(model)) if model.provider_id == provider.id => Some(model),
        (Some(_), Some(_)) => None,
        (_, model) => model,
    };

    Ok(())
}

pub fn inject_runtime_context(
    context: &mut HashMap<String, String>,
    method: &str,
    path: &str,
    headers: &HeaderMap,
    provider: Option<&ProviderConfig>,
    model: Option<&ModelConfig>,
    route: Option<&RouteConfig>,
) {
    context.insert("method".to_string(), method.to_string());
    context.insert("path".to_string(), path.to_string());

    context.retain(|key, _| {
        !key.starts_with("header.")
            && !matches!(
                key.as_str(),
                "provider.id" | "provider.name" | "model.id" | "route.id"
            )
    });

    for (name, value) in headers {
        if let Ok(value) = value.to_str() {
            context.insert(
                format!("header.{}", name.as_str().to_ascii_lowercase()),
                value.to_string(),
            );
        }
    }

    if let Some(provider) = provider {
        context.insert("provider.id".to_string(), provider.id.clone());
        context.insert("provider.name".to_string(), provider.name.clone());
    }
    if let Some(model) = model {
        context.insert("model.id".to_string(), model.id.clone());
    }
    if let Some(route) = route {
        context.insert("route.id".to_string(), route.id.clone());
    }
}
