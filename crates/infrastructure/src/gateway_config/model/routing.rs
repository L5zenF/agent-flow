use serde::{Deserialize, Serialize};

use super::default_enabled;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    pub base_url: String,
    #[serde(default)]
    pub default_headers: Vec<HeaderConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub id: String,
    pub name: String,
    pub provider_id: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteConfig {
    pub id: String,
    #[serde(default)]
    pub priority: i64,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(alias = "match")]
    pub matcher: String,
    pub provider_id: String,
    #[serde(default)]
    pub model_id: Option<String>,
    #[serde(default)]
    pub path_rewrite: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderRuleConfig {
    pub id: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    pub scope: RuleScope,
    #[serde(default)]
    pub target_id: Option<String>,
    #[serde(default)]
    pub when: Option<String>,
    #[serde(default)]
    pub actions: Vec<HeaderActionConfig>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuleScope {
    Global,
    Provider,
    Model,
    Route,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HeaderActionConfig {
    Set { name: String, value: String },
    Remove { name: String },
    Copy { from: String, to: String },
    SetIfAbsent { name: String, value: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderConfig {
    pub name: String,
    #[serde(flatten)]
    pub value: HeaderValueConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HeaderValueConfig {
    Encrypted {
        value: String,
        encrypted: bool,
        #[serde(default)]
        secret_env: Option<String>,
    },
    Plain {
        value: String,
    },
}
