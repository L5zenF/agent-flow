use serde::Deserialize;

wit_bindgen::generate!({
    world: "proxy-node-plugin",
});

use exports::proxy_tools::proxy_node_plugin::node_plugin::{
    ContextEntry, ContextPatch, ContextPatchOp, ExecuteError, ExecuteInput, ExecuteOutput, Guest,
    JsonDocument, LogEntry, LogLevel,
};

#[derive(Debug, Default, Deserialize)]
struct SelectionConfig {
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

        if provider_id.is_empty() && model_id.is_empty() {
            return Err(ExecuteError::InvalidInput(
                "selection step requires provider_id or model_id".to_string(),
            ));
        }

        let mut ops = Vec::new();
        if !provider_id.is_empty() {
            ops.push(ContextPatchOp::Set(ContextEntry {
                key: "selection.provider_id".to_string(),
                value: provider_id.clone(),
            }));
        }
        if !model_id.is_empty() {
            ops.push(ContextPatchOp::Set(ContextEntry {
                key: "selection.model_id".to_string(),
                value: model_id.clone(),
            }));
        }

        let message = config.log_message.unwrap_or_else(|| {
            if !provider_id.is_empty() && !model_id.is_empty() {
                format!("selected provider={} model={}", provider_id, model_id)
            } else if !provider_id.is_empty() {
                format!("selected provider={}", provider_id)
            } else {
                format!("selected model={}", model_id)
            }
        });

        Ok(ExecuteOutput {
            context_patch: Some(ContextPatch { ops }),
            header_ops: Vec::new(),
            path_rewrite: None,
            next_port: None,
            logs: vec![LogEntry {
                level: LogLevel::Info,
                message,
            }],
        })
    }
}

fn parse_config(config: Option<&JsonDocument>) -> Result<SelectionConfig, ExecuteError> {
    match config {
        Some(document) => serde_json::from_str(&document.json)
            .map_err(|error| ExecuteError::InvalidInput(format!("invalid node config JSON: {error}"))),
        None => Ok(SelectionConfig::default()),
    }
}

export!(Component);
