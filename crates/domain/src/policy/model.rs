use std::collections::HashMap;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use super::{evaluate_expression, render_template};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleScope {
    Global,
    Provider,
    Model,
    Route,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeaderAction {
    Set { name: String, value: String },
    Remove { name: String },
    Copy { from: String, to: String },
    SetIfAbsent { name: String, value: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderRule {
    pub id: String,
    pub enabled: bool,
    pub scope: RuleScope,
    pub target_id: Option<String>,
    pub when: Option<String>,
    pub actions: Vec<HeaderAction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderPolicy {
    rules: Vec<HeaderRule>,
}

impl HeaderPolicy {
    pub fn new(rules: Vec<HeaderRule>) -> Self {
        Self { rules }
    }

    pub fn resolve(
        &self,
        provider_defaults: &[(String, String)],
        request: &HeaderPolicyRequest<'_>,
    ) -> Result<Vec<(String, String)>, PolicyError> {
        let mut resolved = HashMap::<String, String>::new();
        for (name, value) in provider_defaults {
            resolved.insert(name.to_ascii_lowercase(), value.clone());
        }

        for rule in self.ordered_rules(request) {
            if !rule.enabled {
                continue;
            }
            if let Some(condition) = rule.when.as_deref()
                && !evaluate_expression(condition, request)?
            {
                continue;
            }
            apply_actions(&mut resolved, &rule.actions, request)?;
        }

        Ok(resolved.into_iter().collect())
    }

    fn ordered_rules<'a>(&'a self, request: &HeaderPolicyRequest<'_>) -> Vec<&'a HeaderRule> {
        let mut global = Vec::new();
        let mut provider = Vec::new();
        let mut model = Vec::new();
        let mut route = Vec::new();

        for rule in &self.rules {
            match rule.scope {
                RuleScope::Global => global.push(rule),
                RuleScope::Provider
                    if request
                        .provider_id
                        .map(|id| rule.target_id.as_deref() == Some(id))
                        .unwrap_or(false) =>
                {
                    provider.push(rule)
                }
                RuleScope::Model
                    if request
                        .model_id
                        .map(|id| rule.target_id.as_deref() == Some(id))
                        .unwrap_or(false) =>
                {
                    model.push(rule)
                }
                RuleScope::Route
                    if request
                        .route_id
                        .map(|id| rule.target_id.as_deref() == Some(id))
                        .unwrap_or(false) =>
                {
                    route.push(rule)
                }
                _ => {}
            }
        }

        global
            .into_iter()
            .chain(provider)
            .chain(model)
            .chain(route)
            .collect()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HeaderPolicyRequest<'a> {
    pub method: &'a str,
    pub path: &'a str,
    pub headers: &'a HashMap<String, String>,
    pub context: &'a HashMap<String, String>,
    pub provider_id: Option<&'a str>,
    pub provider_name: Option<&'a str>,
    pub model_id: Option<&'a str>,
    pub route_id: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyError {
    InvalidExpression(String),
    InvalidTemplate(String),
    UnsupportedValueSource(String),
    MissingRequestHeader(String),
    MissingContextValue(String),
    MissingEnvironmentValue(String),
}

impl Display for PolicyError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidExpression(message) => formatter.write_str(message),
            Self::InvalidTemplate(message) => formatter.write_str(message),
            Self::UnsupportedValueSource(source) => {
                write!(formatter, "unsupported value source '{source}'")
            }
            Self::MissingRequestHeader(key) => {
                write!(formatter, "request header '{key}' is unavailable")
            }
            Self::MissingContextValue(key) => {
                write!(formatter, "context value '{key}' is unavailable")
            }
            Self::MissingEnvironmentValue(key) => {
                write!(formatter, "environment variable '{key}' is not set")
            }
        }
    }
}

impl Error for PolicyError {}

fn apply_actions(
    resolved: &mut HashMap<String, String>,
    actions: &[HeaderAction],
    request: &HeaderPolicyRequest<'_>,
) -> Result<(), PolicyError> {
    for action in actions {
        match action {
            HeaderAction::Set { name, value } => {
                resolved.insert(name.to_ascii_lowercase(), render_template(value, request)?);
            }
            HeaderAction::Remove { name } => {
                resolved.remove(&name.to_ascii_lowercase());
            }
            HeaderAction::Copy { from, to } => {
                let source = request
                    .headers
                    .get(&from.to_ascii_lowercase())
                    .ok_or_else(|| PolicyError::MissingRequestHeader(from.clone()))?;
                resolved.insert(to.to_ascii_lowercase(), source.clone());
            }
            HeaderAction::SetIfAbsent { name, value } => {
                resolved
                    .entry(name.to_ascii_lowercase())
                    .or_insert(render_template(value, request)?);
            }
        }
    }
    Ok(())
}
