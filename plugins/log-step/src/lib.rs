use serde::Deserialize;
use std::collections::BTreeMap;

wit_bindgen::generate!({
    world: "proxy-node-plugin",
});

use exports::proxy_tools::proxy_node_plugin::node_plugin::{
    ExecuteError, ExecuteInput, ExecuteOutput, Guest, JsonDocument, LogEntry, LogLevel,
};

#[derive(Debug, Default, Deserialize)]
struct LogStepConfig {
    log_message: Option<String>,
    level: Option<String>,
}

struct Component;

impl Guest for Component {
    fn execute(input: ExecuteInput) -> Result<ExecuteOutput, ExecuteError> {
        let config = parse_config(input.node_config.config.as_ref())?;
        let headers = header_map(&input);
        let context = context_map(&input);
        let message_template = config
            .log_message
            .unwrap_or_else(|| "log-step method=${method} path=${path}".to_string());
        let logs = vec![LogEntry {
            level: parse_level(config.level.as_deref()),
            message: render_template(&message_template, &input, &headers, &context),
        }];

        Ok(ExecuteOutput {
            context_patch: None,
            header_ops: Vec::new(),
            path_rewrite: None,
            next_port: None,
            logs,
        })
    }
}

fn parse_config(config: Option<&JsonDocument>) -> Result<LogStepConfig, ExecuteError> {
    match config {
        Some(document) => serde_json::from_str(&document.json)
            .map_err(|error| ExecuteError::InvalidInput(format!("invalid node config JSON: {error}"))),
        None => Ok(LogStepConfig::default()),
    }
}

fn parse_level(level: Option<&str>) -> LogLevel {
    match level.map(str::trim).filter(|value| !value.is_empty()) {
        Some("debug") => LogLevel::Debug,
        Some("warn") => LogLevel::Warn,
        Some("error") => LogLevel::Error,
        _ => LogLevel::Info,
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
