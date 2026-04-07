use serde::Deserialize;
use std::collections::BTreeMap;

wit_bindgen::generate!({
    world: "proxy-node-plugin",
});

use exports::proxy_tools::proxy_node_plugin::node_plugin::{
    ContextEntry, ContextPatch, ContextPatchOp, ExecuteError, ExecuteInput, ExecuteOutput, Guest,
    JsonDocument, LogEntry, LogLevel,
};

#[derive(Debug, Default, Deserialize)]
struct SelectModelConfig {
    provider_id: Option<String>,
    model_id: Option<String>,
    log_message: Option<String>,
}

struct Component;

impl Guest for Component {
    fn execute(input: ExecuteInput) -> Result<ExecuteOutput, ExecuteError> {
        let config = parse_config(input.node_config.config.as_ref())?;
        let provider_id = config.provider_id.unwrap_or_default().trim().to_string();
        let model_id = config.model_id.unwrap_or_default().trim().to_string();
        if provider_id.is_empty() || model_id.is_empty() {
            return Err(ExecuteError::InvalidInput(
                "select-model requires non-empty provider_id and model_id".to_string(),
            ));
        }

        let headers = header_map(&input);
        let context = context_map(&input);
        let message_template = config.log_message.unwrap_or_else(|| {
            "selected provider=${ctx.selection.provider_id} model=${ctx.selection.model_id}".to_string()
        });
        let logs = vec![LogEntry {
            level: LogLevel::Info,
            message: render_template(&message_template, &input, &headers, &context),
        }];

        Ok(ExecuteOutput {
            context_patch: Some(ContextPatch {
                ops: vec![
                    ContextPatchOp::Set(ContextEntry {
                        key: "selection.provider_id".to_string(),
                        value: provider_id,
                    }),
                    ContextPatchOp::Set(ContextEntry {
                        key: "selection.model_id".to_string(),
                        value: model_id,
                    }),
                ],
            }),
            header_ops: Vec::new(),
            path_rewrite: None,
            next_port: None,
            logs,
        })
    }
}

fn parse_config(config: Option<&JsonDocument>) -> Result<SelectModelConfig, ExecuteError> {
    match config {
        Some(document) => serde_json::from_str(&document.json)
            .map_err(|error| ExecuteError::InvalidInput(format!("invalid node config JSON: {error}"))),
        None => Ok(SelectModelConfig::default()),
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
