use std::path::{Path, PathBuf};

use application::{
    WorkflowIndexEntryInput, ensure_active_workflow_loaded, load_indexed_workflows,
    resolve_indexed_workflow_path, resolve_indexed_workflows_dir,
};

use crate::atomic_store::write_toml_atomic;
use crate::gateway_config::legacy::{is_not_found_error, uses_synthesized_legacy_workflow_index};
use crate::gateway_config::model::{
    GatewayConfig, LoadedWorkflowSet, RuntimeState, WorkflowFileConfig,
};
use crate::gateway_config::validate::{
    parse_config, unique_ids, validate_config, validate_rule_graph,
};

pub fn load_config(path: &Path) -> Result<GatewayConfig, Box<dyn std::error::Error>> {
    let raw = std::fs::read_to_string(path)?;
    let config = parse_config(&raw)?;
    Ok(config)
}

pub fn load_runtime_state(path: &Path) -> Result<RuntimeState, Box<dyn std::error::Error>> {
    let config = load_config(path)?;
    runtime_state_from_config(path, config)
}

pub fn resolve_workflows_dir(config_path: &Path, config: &GatewayConfig) -> Option<PathBuf> {
    resolve_indexed_workflows_dir(config_path, config.workflows_dir.as_deref())
}

pub fn resolve_workflow_path(
    config_path: &Path,
    config: &GatewayConfig,
    workflow_file: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    resolve_indexed_workflow_path(config_path, config.workflows_dir.as_deref(), workflow_file)
        .map_err(|error| error.to_string().into())
}

pub fn load_workflow_file(path: &Path) -> Result<WorkflowFileConfig, Box<dyn std::error::Error>> {
    let raw = std::fs::read_to_string(path)?;
    Ok(toml::from_str(&raw)?)
}

pub fn save_workflow_file_atomic(
    path: &Path,
    workflow: &WorkflowFileConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    write_toml_atomic(path, workflow)?;
    Ok(())
}

pub fn load_workflow_set(
    config_path: &Path,
    config: &GatewayConfig,
) -> Result<LoadedWorkflowSet, Box<dyn std::error::Error>> {
    let provider_ids = unique_ids(
        config.providers.iter().map(|provider| provider.id.as_str()),
        "provider",
    )?;
    let model_ids = unique_ids(config.models.iter().map(|model| model.id.as_str()), "model")?;
    let workflow_entries = config
        .workflows
        .iter()
        .map(|workflow| WorkflowIndexEntryInput {
            id: workflow.id.clone(),
            file: workflow.file.clone(),
        })
        .collect::<Vec<_>>();
    let allow_legacy_missing_file_fallback = uses_synthesized_legacy_workflow_index(config);
    let by_id = load_indexed_workflows(
        config_path,
        config.workflows_dir.as_deref(),
        &workflow_entries,
        allow_legacy_missing_file_fallback,
        load_workflow_file,
        |error| is_not_found_error(error.as_ref()),
        |workflow_id, workflow_path, loaded| {
            validate_rule_graph(&loaded.workflow, &provider_ids, &model_ids, &config.models)
                .map_err(|error| {
                    format!(
                        "workflow '{}' in '{}' is invalid: {error}",
                        workflow_id,
                        workflow_path.display()
                    )
                })
        },
    )
    .map_err(|error| error.to_string())?;

    ensure_active_workflow_loaded(
        config.active_workflow_id.as_deref(),
        !config.workflows.is_empty(),
        &by_id,
        allow_legacy_missing_file_fallback,
    )
    .map_err(|error| error.to_string())?;

    Ok(LoadedWorkflowSet {
        summaries: config.workflows.clone(),
        by_id,
        active_workflow_id: config.active_workflow_id.clone(),
        legacy_rule_graph: config.rule_graph.clone(),
    })
}

pub fn runtime_state_from_config(
    config_path: &Path,
    config: GatewayConfig,
) -> Result<RuntimeState, Box<dyn std::error::Error>> {
    let workflow_set = load_workflow_set(config_path, &config)?;
    Ok(RuntimeState {
        config,
        workflow_set,
    })
}

pub fn save_config_atomic(
    path: &Path,
    config: &GatewayConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let normalized = crate::gateway_config::normalize_legacy_rule_graph(config.clone());
    validate_config(&normalized)?;
    write_toml_atomic(path, &normalized)?;
    Ok(())
}
