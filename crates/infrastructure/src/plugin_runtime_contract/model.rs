#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeRequestHeader {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RuntimeCapabilityKind {
    Log,
    Fs,
    Network,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeCapabilityDeclaration {
    pub kind: RuntimeCapabilityKind,
    pub required: bool,
    pub scope: Option<String>,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeCapabilityGrant {
    pub kind: RuntimeCapabilityKind,
    pub allowed: bool,
    pub scope: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub supported_output_ports: Vec<String>,
    pub default_config_schema_hints_json: Option<String>,
    pub capabilities: Vec<RuntimeCapabilityDeclaration>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeNodeConfig {
    pub manifest: RuntimePluginManifest,
    pub grants: Vec<RuntimeCapabilityGrant>,
    pub config_json: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeExecuteInput {
    pub request_method: String,
    pub current_path: String,
    pub request_headers: Vec<RuntimeRequestHeader>,
    pub workflow_context: Vec<(String, String)>,
    pub selected_provider_id: String,
    pub selected_model_id: String,
    pub node_config: RuntimeNodeConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeContextPatchOp {
    Set { key: String, value: String },
    Remove { key: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeHeaderOp {
    Set { name: String, value: String },
    Append { name: String, value: String },
    Remove { name: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeLogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeLogEntry {
    pub level: RuntimeLogLevel,
    pub message: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuntimeExecuteOutput {
    pub context_ops: Vec<RuntimeContextPatchOp>,
    pub header_ops: Vec<RuntimeHeaderOp>,
    pub path_rewrite: Option<String>,
    pub next_port: Option<String>,
    pub logs: Vec<RuntimeLogEntry>,
}
