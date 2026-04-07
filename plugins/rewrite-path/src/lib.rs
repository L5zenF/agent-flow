use serde::Deserialize;
use std::collections::BTreeMap;

wit_bindgen::generate!({
    world: "proxy-node-plugin",
});

use exports::proxy_tools::proxy_node_plugin::node_plugin::{
    ExecuteError, ExecuteInput, ExecuteOutput, Guest, JsonDocument, LogEntry, LogLevel,
    PathRewrite,
};

#[derive(Debug, Default, Deserialize)]
struct RewritePathConfig {
    path_rewrite: Option<String>,
    log_message: Option<String>,
}

struct Component;

impl Guest for Component {
    fn execute(input: ExecuteInput) -> Result<ExecuteOutput, ExecuteError> {
        let config = parse_config(input.node_config.config.as_ref())?;
        let headers = header_map(&input);
        let context = context_map(&input);

        let path_rewrite = config
            .path_rewrite
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|path| PathRewrite {
                path: render_template(path, &input, &headers, &context),
            });

        let logs = config
            .log_message
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(|message| {
                vec![LogEntry {
                    level: LogLevel::Info,
                    message: render_template(message, &input, &headers, &context),
                }]
            })
            .unwrap_or_default();

        Ok(ExecuteOutput {
            context_patch: None,
            header_ops: Vec::new(),
            path_rewrite,
            next_port: None,
            logs,
        })
    }
}

fn parse_config(config: Option<&JsonDocument>) -> Result<RewritePathConfig, ExecuteError> {
    match config {
        Some(document) => serde_json::from_str(&document.json)
            .map_err(|error| ExecuteError::InvalidInput(format!("invalid node config JSON: {error}"))),
        None => Ok(RewritePathConfig::default()),
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
