use std::collections::BTreeMap;
use std::fmt::Display;
use std::path::{Path, PathBuf};

use crate::ApplicationError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowIndexEntryInput {
    pub id: String,
    pub file: String,
}

pub fn resolve_workflows_dir(config_path: &Path, workflows_dir: Option<&str>) -> Option<PathBuf> {
    workflows_dir.map(|relative| {
        config_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(relative)
    })
}

pub fn resolve_workflow_path(
    config_path: &Path,
    workflows_dir: Option<&str>,
    workflow_file: &str,
) -> Result<PathBuf, ApplicationError> {
    let workflows_dir = resolve_workflows_dir(config_path, workflows_dir).ok_or_else(|| {
        ApplicationError::Validation(
            "workflows_dir must be set when workflows are present".to_string(),
        )
    })?;
    Ok(workflows_dir.join(workflow_file))
}

pub fn load_indexed_workflows<T, LoadErr, ValidateErr>(
    config_path: &Path,
    workflows_dir: Option<&str>,
    workflows: &[WorkflowIndexEntryInput],
    allow_legacy_missing_file_fallback: bool,
    load_file: impl Fn(&Path) -> Result<T, LoadErr>,
    is_not_found: impl Fn(&LoadErr) -> bool,
    validate: impl Fn(&str, &Path, &T) -> Result<(), ValidateErr>,
) -> Result<BTreeMap<String, T>, ApplicationError>
where
    LoadErr: Display,
    ValidateErr: Display,
{
    let mut by_id = BTreeMap::new();
    if workflows.is_empty() {
        return Ok(by_id);
    }
    let workflows_dir = resolve_workflows_dir(config_path, workflows_dir).ok_or_else(|| {
        ApplicationError::Validation(
            "workflows_dir must be set when workflows are present".to_string(),
        )
    })?;
    for workflow in workflows {
        let workflow_path = workflows_dir.join(&workflow.file);
        let loaded = match load_file(&workflow_path) {
            Ok(loaded) => loaded,
            Err(error) if allow_legacy_missing_file_fallback && is_not_found(&error) => continue,
            Err(error) => {
                return Err(ApplicationError::Validation(format!(
                    "failed to load workflow '{}' from '{}': {error}",
                    workflow.id,
                    workflow_path.display()
                )));
            }
        };
        validate(&workflow.id, &workflow_path, &loaded)
            .map_err(|error| ApplicationError::Validation(error.to_string()))?;
        by_id.insert(workflow.id.clone(), loaded);
    }
    Ok(by_id)
}

pub fn ensure_active_workflow_loaded(
    active_workflow_id: Option<&str>,
    has_indexed_workflows: bool,
    loaded_workflow_ids: &BTreeMap<String, impl Sized>,
    allow_legacy_missing_file_fallback: bool,
) -> Result<(), ApplicationError> {
    if let Some(active_workflow_id) = active_workflow_id {
        if has_indexed_workflows
            && !loaded_workflow_ids.contains_key(active_workflow_id)
            && !allow_legacy_missing_file_fallback
        {
            return Err(ApplicationError::Validation(format!(
                "active workflow '{}' could not be loaded from indexed workflow files",
                active_workflow_id
            )));
        }
    }
    Ok(())
}
