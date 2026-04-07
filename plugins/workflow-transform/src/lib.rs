use serde::Deserialize;
use std::collections::BTreeMap;

wit_bindgen::generate!({
    world: "proxy-node-plugin",
});

use exports::proxy_tools::proxy_node_plugin::node_plugin::{
    ContextEntry, ContextPatch, ContextPatchOp, ExecuteError, ExecuteInput, ExecuteOutput, Guest,
    HeaderOp, JsonDocument, LogEntry, LogLevel, NextPort, PathRewrite, RequestHeader,
};

#[derive(Debug, Default, Deserialize)]
struct TransformConfig {
    path_rewrite: Option<String>,
    next_port: Option<String>,
    log_message: Option<String>,
    #[serde(default)]
    set_headers: Vec<HeaderMutation>,
    #[serde(default)]
    set_context: Vec<ContextMutation>,
}

#[derive(Debug, Deserialize)]
struct HeaderMutation {
    op: String,
    name: Option<String>,
    value: Option<String>,
    from: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ContextMutation {
    key: String,
    value: String,
}

struct Component;

impl Guest for Component {
    fn execute(input: ExecuteInput) -> Result<ExecuteOutput, ExecuteError> {
        let config = parse_config(input.node_config.config.as_ref())?;
        let headers = header_map(&input);
        let context = context_map(&input);

        let header_ops = config
            .set_headers
            .iter()
            .map(|mutation| build_header_op(mutation, &input, &headers, &context))
            .collect::<Result<Vec<_>, _>>()?;

        let context_ops = config
            .set_context
            .iter()
            .map(|mutation| {
                Ok(ContextPatchOp::Set(ContextEntry {
                    key: mutation.key.trim().to_string(),
                    value: render_template(&mutation.value, &input, &headers, &context),
                }))
            })
            .collect::<Result<Vec<_>, ExecuteError>>()?;

        let logs = config
            .log_message
            .as_deref()
            .filter(|message| !message.trim().is_empty())
            .map(|message| {
                vec![LogEntry {
                    level: LogLevel::Info,
                    message: render_template(message, &input, &headers, &context),
                }]
            })
            .unwrap_or_default();

        Ok(ExecuteOutput {
            context_patch: (!context_ops.is_empty()).then_some(ContextPatch { ops: context_ops }),
            header_ops,
            path_rewrite: config.path_rewrite.as_deref().map(|path| PathRewrite {
                path: render_template(path, &input, &headers, &context),
            }),
            next_port: config
                .next_port
                .as_deref()
                .filter(|port| !port.trim().is_empty())
                .map(|port| NextPort {
                    port: port.trim().to_string(),
                }),
            logs,
        })
    }
}

fn parse_config(config: Option<&JsonDocument>) -> Result<TransformConfig, ExecuteError> {
    match config {
        Some(document) => serde_json::from_str(&document.json)
            .map_err(|error| ExecuteError::InvalidInput(format!("invalid node config JSON: {error}"))),
        None => Ok(TransformConfig::default()),
    }
}

fn header_map(input: &ExecuteInput) -> BTreeMap<String, String> {
    input
        .request_headers
        .iter()
        .map(|header| (header.name.to_ascii_lowercase(), header.value.clone()))
        .collect()
}

fn context_map(input: &ExecuteInput) -> BTreeMap<String, String> {
    input
        .workflow_context
        .iter()
        .map(|entry| (entry.key.clone(), entry.value.clone()))
        .collect()
}

fn build_header_op(
    mutation: &HeaderMutation,
    input: &ExecuteInput,
    headers: &BTreeMap<String, String>,
    context: &BTreeMap<String, String>,
) -> Result<HeaderOp, ExecuteError> {
    match mutation.op.trim() {
        "set" => Ok(HeaderOp::Set(RequestHeader {
            name: required_name(mutation)?,
            value: render_template(required_value(mutation)?, input, headers, context),
        })),
        "append" => Ok(HeaderOp::Append(RequestHeader {
            name: required_name(mutation)?,
            value: render_template(required_value(mutation)?, input, headers, context),
        })),
        "remove" => Ok(HeaderOp::Remove(required_name(mutation)?)),
        "copy" => {
            let from = mutation
                .from
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| ExecuteError::InvalidInput("copy op requires 'from'".to_string()))?;
            let target = required_name(mutation)?;
            let copied = headers.get(&from.to_ascii_lowercase()).cloned().unwrap_or_default();
            Ok(HeaderOp::Set(RequestHeader {
                name: target,
                value: copied,
            }))
        }
        "set_if_absent" => {
            let name = required_name(mutation)?;
            let exists = headers.contains_key(&name.to_ascii_lowercase());
            if exists {
                Ok(HeaderOp::Set(RequestHeader {
                    name: name.clone(),
                    value: headers
                        .get(&name.to_ascii_lowercase())
                        .cloned()
                        .unwrap_or_default(),
                }))
            } else {
                Ok(HeaderOp::Set(RequestHeader {
                    name,
                    value: render_template(required_value(mutation)?, input, headers, context),
                }))
            }
        }
        other => Err(ExecuteError::InvalidInput(format!(
            "unsupported header op '{other}'"
        ))),
    }
}

fn required_name(mutation: &HeaderMutation) -> Result<String, ExecuteError> {
    mutation
        .name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| ExecuteError::InvalidInput("header op requires 'name'".to_string()))
}

fn required_value(mutation: &HeaderMutation) -> Result<&str, ExecuteError> {
    mutation
        .value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ExecuteError::InvalidInput("header op requires 'value'".to_string()))
}

fn render_template(
    template: &str,
    input: &ExecuteInput,
    headers: &BTreeMap<String, String>,
    context: &BTreeMap<String, String>,
) -> String {
    let mut rendered = template.to_string();
    for (token, value) in [
        ("${path}", input.current_path.as_str()),
        ("${method}", input.request_method.as_str()),
        ("${provider.id}", input.selected_provider_id.as_str()),
        ("${model.id}", input.selected_model_id.as_str()),
    ] {
        rendered = rendered.replace(token, value);
    }

    for (key, value) in headers {
        rendered = rendered.replace(&format!("${{header.{key}}}"), value);
    }

    for (key, value) in context {
        rendered = rendered.replace(&format!("${{ctx.{key}}}"), value);
    }

    rendered
}

export!(Component);
