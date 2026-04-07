use serde::Deserialize;

wit_bindgen::generate!({
    world: "proxy-node-plugin",
});

use exports::proxy_tools::proxy_node_plugin::node_plugin::{
    ContextEntry, ContextPatch, ContextPatchOp, ExecuteError, ExecuteInput, ExecuteOutput, Guest,
    JsonDocument, LogEntry, LogLevel, NextPort, NodeConfig, PluginManifest,
};

#[derive(Debug, Default, Deserialize)]
struct ConditionConfig {
    expr: Option<String>,
}

struct Component;

impl Guest for Component {
    fn execute(input: ExecuteInput) -> Result<ExecuteOutput, ExecuteError> {
        let config = parse_config(input.node_config.config.as_ref())?;
        let expr = config.expr.unwrap_or_default();
        let matched = evaluate_expression(&expr, &input);

        Ok(ExecuteOutput {
            context_patch: Some(ContextPatch {
                ops: vec![ContextPatchOp::Set(ContextEntry {
                    key: "condition.last_result".to_string(),
                    value: if matched { "true" } else { "false" }.to_string(),
                })],
            }),
            header_ops: Vec::new(),
            path_rewrite: None,
            next_port: Some(NextPort {
                port: if matched { "true" } else { "false" }.to_string(),
            }),
            logs: vec![LogEntry {
                level: LogLevel::Info,
                message: format!(
                    "condition-evaluator evaluated expr '{}' => {}",
                    expr, matched
                ),
            }],
        })
    }
}

fn parse_config(config: Option<&JsonDocument>) -> Result<ConditionConfig, ExecuteError> {
    match config {
        Some(document) => serde_json::from_str(&document.json)
            .map_err(|error| ExecuteError::InvalidInput(format!("invalid node config JSON: {error}"))),
        None => Ok(ConditionConfig::default()),
    }
}

fn evaluate_expression(expr: &str, input: &ExecuteInput) -> bool {
    if expr.trim().is_empty() {
        return false;
    }

    expr.split("&&")
        .map(str::trim)
        .all(|clause| evaluate_clause(clause, input).unwrap_or(false))
}

fn evaluate_clause(clause: &str, input: &ExecuteInput) -> Option<bool> {
    if let Some(expected) = parse_eq(clause, "method") {
        return Some(input.request_method == expected);
    }
    if let Some(expected) = parse_eq(clause, "path") {
        return Some(input.current_path == expected);
    }
    if let Some((header_name, expected)) = parse_header_eq(clause) {
        let actual = input
            .request_headers
            .iter()
            .find(|header| header.name.eq_ignore_ascii_case(&header_name))
            .map(|header| header.value.as_str());
        return Some(actual == Some(expected.as_str()));
    }
    if let Some(expected) = parse_contains(clause, "path") {
        return Some(input.current_path.contains(&expected));
    }

    None
}

fn parse_eq(clause: &str, field: &str) -> Option<String> {
    let (lhs, rhs) = clause.split_once("==")?;
    if lhs.trim() != field {
        return None;
    }
    parse_quoted(rhs.trim())
}

fn parse_header_eq(clause: &str) -> Option<(String, String)> {
    let (lhs, rhs) = clause.split_once("==")?;
    let lhs = lhs.trim();
    let header_prefix = "ctx.header.";
    let header_name = lhs.strip_prefix(header_prefix)?.trim().to_string();
    Some((header_name, parse_quoted(rhs.trim())?))
}

fn parse_contains(clause: &str, field: &str) -> Option<String> {
    let prefix = format!("{field}.contains(");
    let suffix = ")";
    let inner = clause.trim().strip_prefix(&prefix)?.strip_suffix(suffix)?;
    parse_quoted(inner.trim())
}

fn parse_quoted(value: &str) -> Option<String> {
    if value.len() < 2 {
        return None;
    }
    if value.starts_with('"') && value.ends_with('"') {
        return Some(value[1..value.len() - 1].to_string());
    }
    if value.starts_with('\'') && value.ends_with('\'') {
        return Some(value[1..value.len() - 1].to_string());
    }
    None
}

fn _empty_input() -> ExecuteInput {
    ExecuteInput {
        request_method: String::new(),
        current_path: String::new(),
        request_headers: Vec::new(),
        workflow_context: Vec::new(),
        selected_provider_id: String::new(),
        selected_model_id: String::new(),
        node_config: NodeConfig {
            manifest: PluginManifest {
                id: String::new(),
                name: String::new(),
                version: String::new(),
                description: String::new(),
                supported_output_ports: Vec::new(),
                default_config_schema_hints: None,
                capabilities: Vec::new(),
            },
            grants: Vec::new(),
            config: None,
        },
    }
}

export!(Component);
