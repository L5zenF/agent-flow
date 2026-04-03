use std::collections::HashSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    #[serde(default = "default_listen")]
    pub listen: String,
    #[serde(default = "default_admin_listen")]
    pub admin_listen: String,
    #[serde(default)]
    pub default_secret_env: Option<String>,
    #[serde(default)]
    pub providers: Vec<ProviderConfig>,
    #[serde(default)]
    pub models: Vec<ModelConfig>,
    #[serde(default)]
    pub routes: Vec<RouteConfig>,
    #[serde(default)]
    pub header_rules: Vec<HeaderRuleConfig>,
}

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
    Plain { value: String },
}

pub fn load_config(path: &Path) -> Result<GatewayConfig, Box<dyn std::error::Error>> {
    let raw = std::fs::read_to_string(path)?;
    parse_config(&raw)
}

pub fn save_config_atomic(
    path: &Path,
    config: &GatewayConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_config(config)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let serialized = toml::to_string_pretty(config)?;
    let temp_path = path.with_extension("toml.tmp");
    std::fs::write(&temp_path, serialized)?;
    std::fs::rename(temp_path, path)?;
    Ok(())
}

pub fn parse_config(raw: &str) -> Result<GatewayConfig, Box<dyn std::error::Error>> {
    let config: GatewayConfig = toml::from_str(raw)?;
    validate_config(&config)?;
    Ok(config)
}

pub fn validate_config(config: &GatewayConfig) -> Result<(), Box<dyn std::error::Error>> {
    let provider_ids = unique_ids(
        config.providers.iter().map(|provider| provider.id.as_str()),
        "provider",
    )?;
    let model_ids = unique_ids(config.models.iter().map(|model| model.id.as_str()), "model")?;
    let route_ids = unique_ids(config.routes.iter().map(|route| route.id.as_str()), "route")?;
    unique_ids(
        config.header_rules.iter().map(|rule| rule.id.as_str()),
        "header_rule",
    )?;

    for provider in &config.providers {
        if provider.id.trim().is_empty() {
            return Err("provider id cannot be empty".into());
        }
        if provider.base_url.trim().is_empty() {
            return Err(format!("provider '{}' base_url cannot be empty", provider.id).into());
        }
    }

    for model in &config.models {
        if !provider_ids.contains(model.provider_id.as_str()) {
            return Err(format!(
                "model '{}' references missing provider '{}'",
                model.id, model.provider_id
            )
            .into());
        }
    }

    for route in &config.routes {
        if route.matcher.trim().is_empty() {
            return Err(format!("route '{}' matcher cannot be empty", route.id).into());
        }
        if !provider_ids.contains(route.provider_id.as_str()) {
            return Err(format!(
                "route '{}' references missing provider '{}'",
                route.id, route.provider_id
            )
            .into());
        }
        if let Some(model_id) = route.model_id.as_deref() {
            if !model_ids.contains(model_id) {
                return Err(format!(
                    "route '{}' references missing model '{}'",
                    route.id, model_id
                )
                .into());
            }
        }
    }

    for rule in &config.header_rules {
        match rule.scope {
            RuleScope::Global => {
                if rule.target_id.is_some() {
                    return Err(format!(
                        "header_rule '{}' must not define target_id for global scope",
                        rule.id
                    )
                    .into());
                }
            }
            RuleScope::Provider => validate_rule_target(rule, &provider_ids, "provider")?,
            RuleScope::Model => validate_rule_target(rule, &model_ids, "model")?,
            RuleScope::Route => validate_rule_target(rule, &route_ids, "route")?,
        }

        if rule.actions.is_empty() {
            return Err(format!("header_rule '{}' must contain at least one action", rule.id).into());
        }
    }

    Ok(())
}

fn validate_rule_target(
    rule: &HeaderRuleConfig,
    ids: &HashSet<&str>,
    kind: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(target_id) = rule.target_id.as_deref() else {
        return Err(format!(
            "header_rule '{}' requires target_id for {kind} scope",
            rule.id
        )
        .into());
    };

    if !ids.contains(target_id) {
        return Err(format!(
            "header_rule '{}' references missing {kind} '{}'",
            rule.id, target_id
        )
        .into());
    }

    Ok(())
}

fn unique_ids<'a>(
    ids: impl IntoIterator<Item = &'a str>,
    kind: &str,
) -> Result<HashSet<&'a str>, Box<dyn std::error::Error>> {
    let mut seen = HashSet::new();
    for id in ids {
        if id.trim().is_empty() {
            return Err(format!("{kind} id cannot be empty").into());
        }
        if !seen.insert(id) {
            return Err(format!("duplicate {kind} id '{id}'").into());
        }
    }
    Ok(seen)
}

fn default_listen() -> String {
    "127.0.0.1:9001".to_string()
}

fn default_admin_listen() -> String {
    "127.0.0.1:9002".to_string()
}

fn default_enabled() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::{parse_config, RuleScope};

    const VALID_CONFIG: &str = r#"
listen = "127.0.0.1:9001"
admin_listen = "127.0.0.1:9002"

[[providers]]
id = "kimi"
name = "Kimi"
base_url = "https://api.kimi.com"

[[providers.default_headers]]
name = "Authorization"
value = "enc:v1:test"
encrypted = true

[[models]]
id = "kimi-k2"
name = "Kimi K2"
provider_id = "kimi"

[[routes]]
id = "chat-default"
priority = 100
enabled = true
matcher = 'path.startsWith("/v1/chat/completions") && method == "POST"'
provider_id = "kimi"
model_id = "kimi-k2"
path_rewrite = "/coding/v1/chat/completions"

[[header_rules]]
id = "inject-model-header"
enabled = true
scope = "model"
target_id = "kimi-k2"
when = 'path.startsWith("/v1/")'

[[header_rules.actions]]
type = "set"
name = "X-Model"
value = "${model.id}"
"#;

    #[test]
    fn parses_structured_gateway_config() {
        let config = parse_config(VALID_CONFIG).expect("valid config should parse");

        assert_eq!(config.providers.len(), 1);
        assert_eq!(config.models.len(), 1);
        assert_eq!(config.routes.len(), 1);
        assert_eq!(config.header_rules.len(), 1);
        assert_eq!(config.header_rules[0].scope, RuleScope::Model);
    }

    #[test]
    fn rejects_missing_provider_reference() {
        let invalid = VALID_CONFIG.replace("provider_id = \"kimi\"", "provider_id = \"missing\"");
        let error = parse_config(&invalid).expect_err("config should reject missing provider");

        assert!(
            error
                .to_string()
                .contains("references missing provider 'missing'"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn rejects_global_rule_with_target() {
        let config = VALID_CONFIG.replace("scope = \"model\"", "scope = \"global\"");
        let error = parse_config(&config).expect_err("global scope should reject target_id");

        assert!(
            error
                .to_string()
                .contains("must not define target_id for global scope"),
            "unexpected error: {error}"
        );
    }
}
