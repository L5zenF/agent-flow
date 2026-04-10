use serde::{Deserialize, Serialize};

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
