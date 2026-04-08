use serde::{Deserialize, Serialize};

use crate::plugin_runtime_contract::model::{
    RuntimeContextPatchOp, RuntimeExecuteInput, RuntimeExecuteOutput, RuntimeHeaderOp,
    RuntimeLogEntry, RuntimeLogLevel,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeRunnerInput<'a> {
    pub method: &'a str,
    pub path: &'a str,
    pub headers: std::collections::BTreeMap<String, String>,
    pub context: std::collections::BTreeMap<String, String>,
    pub provider: Option<CodeRunnerProvider<'a>>,
    pub model: Option<CodeRunnerModel<'a>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeRunnerProvider<'a> {
    pub id: &'a str,
    pub name: &'a str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeRunnerModel<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub provider_id: &'a str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreCodeRunnerRequest<'a> {
    pub code: String,
    pub input: CodeRunnerInput<'a>,
}

#[derive(Debug, Deserialize)]
pub struct CoreCodeRunnerResponse {
    pub ok: bool,
    #[serde(default)]
    pub json: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeRunnerOutput {
    #[serde(default)]
    pub context_patch: Vec<CodeRunnerContextPatchOp>,
    #[serde(default)]
    pub header_ops: Vec<CodeRunnerHeaderOp>,
    #[serde(default)]
    pub path_rewrite: Option<String>,
    #[serde(default)]
    pub next_port: Option<String>,
    #[serde(default)]
    pub logs: Vec<CodeRunnerLogEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum CodeRunnerContextPatchOp {
    Set { key: String, value: String },
    Remove { key: String },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum CodeRunnerHeaderOp {
    Set { name: String, value: String },
    Append { name: String, value: String },
    Remove { name: String },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CodeRunnerLogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Deserialize)]
pub struct CodeRunnerLogEntry {
    pub level: CodeRunnerLogLevel,
    pub message: String,
}

pub fn build_code_runner_input(input: &RuntimeExecuteInput) -> CodeRunnerInput<'_> {
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

pub fn normalize_code_runner_source(source: &str) -> String {
    let trimmed = source.trim_start();
    if trimmed.starts_with("export function run") {
        source.replacen("export function run", "function run", 1)
    } else {
        source.to_string()
    }
}

pub fn parse_code_runner_output(node_id: &str, json: &str) -> Result<RuntimeExecuteOutput, String> {
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
