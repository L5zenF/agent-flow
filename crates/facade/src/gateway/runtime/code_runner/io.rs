use std::collections::HashMap;

use axum::http::HeaderMap;
use infrastructure::plugin_runtime_contract::{
    CodeRunnerContextPatchOp, CodeRunnerHeaderOp, CodeRunnerInput, CodeRunnerLogLevel,
    CodeRunnerModel, CodeRunnerOutput, CodeRunnerProvider, RuntimeContextPatchOp,
    RuntimeExecuteInput, RuntimeExecuteOutput, RuntimeHeaderOp, RuntimeLogEntry, RuntimeLogLevel,
};

use super::super::component_runtime::log_runtime_output;
use super::empty_runtime_execute_input;
use crate::config::{ModelConfig, ProviderConfig};

#[allow(clippy::too_many_arguments)]
pub(super) fn build_runtime_input(
    node_id: &str,
    method: &str,
    resolved_path: &str,
    headers: &HeaderMap,
    workflow_context: &HashMap<String, String>,
    outgoing_headers: &HashMap<String, Vec<String>>,
    selected_provider: Option<&ProviderConfig>,
    selected_model: Option<&ModelConfig>,
) -> RuntimeExecuteInput {
    let mut input = empty_runtime_execute_input(node_id);
    input.request_method = method.to_string();
    input.current_path = resolved_path.to_string();
    input.request_headers = super::super::wasi::current_request_headers(headers, outgoing_headers);
    input.workflow_context = workflow_context
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    input.selected_provider_id = selected_provider
        .map(|provider| provider.id.clone())
        .unwrap_or_default();
    input.selected_model_id = selected_model
        .map(|model| model.id.clone())
        .unwrap_or_default();
    input
}

pub(super) fn build_code_runner_input(input: &RuntimeExecuteInput) -> CodeRunnerInput<'_> {
    CodeRunnerInput {
        method: input.request_method.as_str(),
        path: input.current_path.as_str(),
        headers: input
            .request_headers
            .iter()
            .map(|header| (header.name.to_ascii_lowercase(), header.value.clone()))
            .collect(),
        context: input.workflow_context.iter().cloned().collect(),
        provider: (!input.selected_provider_id.is_empty()).then(|| CodeRunnerProvider {
            id: input.selected_provider_id.as_str(),
            name: input.selected_provider_id.as_str(),
        }),
        model: (!input.selected_model_id.is_empty()).then(|| CodeRunnerModel {
            id: input.selected_model_id.as_str(),
            name: input.selected_model_id.as_str(),
            provider_id: input.selected_provider_id.as_str(),
        }),
    }
}

pub(super) fn normalize_code_runner_source(source: &str) -> String {
    let trimmed = source.trim_start();
    if trimmed.starts_with("export function run") {
        source.replacen("export function run", "function run", 1)
    } else {
        source.to_string()
    }
}

pub(super) fn parse_code_runner_output(
    node_id: &str,
    json: &str,
) -> Result<RuntimeExecuteOutput, String> {
    let output = serde_json::from_str::<CodeRunnerOutput>(json).map_err(|error| {
        format!("code_runner node '{node_id}' returned invalid output JSON: {error}")
    })?;
    Ok(RuntimeExecuteOutput {
        context_ops: output
            .context_patch
            .into_iter()
            .map(|op| match op {
                CodeRunnerContextPatchOp::Set { key, value } => {
                    RuntimeContextPatchOp::Set { key, value }
                }
                CodeRunnerContextPatchOp::Remove { key } => RuntimeContextPatchOp::Remove { key },
            })
            .collect(),
        header_ops: output
            .header_ops
            .into_iter()
            .map(|op| match op {
                CodeRunnerHeaderOp::Set { name, value } => RuntimeHeaderOp::Set { name, value },
                CodeRunnerHeaderOp::Append { name, value } => {
                    RuntimeHeaderOp::Append { name, value }
                }
                CodeRunnerHeaderOp::Remove { name } => RuntimeHeaderOp::Remove { name },
            })
            .collect(),
        path_rewrite: output.path_rewrite,
        next_port: output.next_port,
        logs: output
            .logs
            .into_iter()
            .map(|log| RuntimeLogEntry {
                level: match log.level {
                    CodeRunnerLogLevel::Debug => RuntimeLogLevel::Debug,
                    CodeRunnerLogLevel::Info => RuntimeLogLevel::Info,
                    CodeRunnerLogLevel::Warn => RuntimeLogLevel::Warn,
                    CodeRunnerLogLevel::Error => RuntimeLogLevel::Error,
                },
                message: log.message,
            })
            .collect(),
    })
}

pub(super) fn apply_output(
    node_id: &str,
    output: &RuntimeExecuteOutput,
    workflow_context: &mut HashMap<String, String>,
    outgoing_headers: &mut HashMap<String, Vec<String>>,
    resolved_path: &mut String,
) {
    log_runtime_output(node_id, None, output);
    for op in &output.context_ops {
        match op {
            RuntimeContextPatchOp::Set { key, value } => {
                workflow_context.insert(key.clone(), value.clone());
            }
            RuntimeContextPatchOp::Remove { key } => {
                workflow_context.remove(key);
            }
        }
    }
    for op in &output.header_ops {
        match op {
            RuntimeHeaderOp::Set { name, value } => {
                outgoing_headers.insert(name.to_ascii_lowercase(), vec![value.clone()]);
            }
            RuntimeHeaderOp::Append { name, value } => {
                outgoing_headers
                    .entry(name.to_ascii_lowercase())
                    .or_default()
                    .push(value.clone());
            }
            RuntimeHeaderOp::Remove { name } => {
                outgoing_headers.remove(&name.to_ascii_lowercase());
            }
        }
    }
    if let Some(path_rewrite) = &output.path_rewrite {
        *resolved_path = path_rewrite.clone();
    }
}
