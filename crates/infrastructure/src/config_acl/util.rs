use std::collections::HashSet;
use std::path::{Component, Path};

use super::InfrastructureAclError;

pub(super) fn unique_ids<'a>(
    ids: impl IntoIterator<Item = &'a str>,
    kind: &str,
) -> Result<HashSet<&'a str>, InfrastructureAclError> {
    let mut seen = HashSet::new();
    for id in ids {
        if id.trim().is_empty() {
            return Err(InfrastructureAclError::Validation(format!(
                "{kind} id cannot be empty"
            )));
        }
        if !seen.insert(id) {
            return Err(InfrastructureAclError::Validation(format!(
                "duplicate {kind} id '{id}'"
            )));
        }
    }
    Ok(seen)
}

pub(super) fn validate_relative_path(
    field: &str,
    value: &str,
) -> Result<(), InfrastructureAclError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(InfrastructureAclError::Validation(format!(
            "{field} cannot be empty"
        )));
    }

    let path = Path::new(value);
    if path.is_absolute() {
        return Err(InfrastructureAclError::Validation(format!(
            "{field} must use relative paths"
        )));
    }
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::Prefix(_) | Component::RootDir
        )
    }) {
        return Err(InfrastructureAclError::Validation(format!(
            "{field} must not contain parent traversal"
        )));
    }

    Ok(())
}

pub(super) fn default_wasm_plugin_timeout_ms() -> u64 {
    20
}
