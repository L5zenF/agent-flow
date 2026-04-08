use std::collections::HashSet;
use std::path::{Component, Path};

use crate::ApplicationError;

pub(super) fn validate_workflow_relative_path(
    field: &str,
    value: &str,
) -> Result<(), ApplicationError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(ApplicationError::Validation(format!(
            "{field} cannot be empty"
        )));
    }

    let path = Path::new(value);
    if path.is_absolute() {
        return Err(ApplicationError::Validation(format!(
            "{field} must use relative paths"
        )));
    }
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::Prefix(_) | Component::RootDir
        )
    }) {
        return Err(ApplicationError::Validation(format!(
            "{field} must not contain parent traversal"
        )));
    }

    Ok(())
}

pub(super) fn unique_ids<'a>(
    ids: impl IntoIterator<Item = &'a str>,
    kind: &str,
) -> Result<HashSet<String>, ApplicationError> {
    let mut seen = HashSet::new();
    for id in ids {
        if id.trim().is_empty() {
            return Err(ApplicationError::Validation(format!(
                "{kind} id cannot be empty"
            )));
        }
        if !seen.insert(id.to_string()) {
            return Err(ApplicationError::Validation(format!(
                "duplicate {kind} id '{id}'"
            )));
        }
    }
    Ok(seen)
}
