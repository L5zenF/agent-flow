use application::{plan_create_workflow, require_workflow};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;

use crate::admin_api::support::{
    default_workflow_document, ensure_active_workflow_file, from_application_workflow_entry,
    invalid_request, to_application_workflow_entries, workflow_admin_error, workflow_document,
    workflow_summary,
};
use crate::admin_api::types::{AdminState, CreateWorkflowRequest, WorkflowSummary};
use crate::config::{
    WorkflowFileConfig, resolve_workflow_path, runtime_state_from_config, save_config_atomic,
    save_workflow_file_atomic,
};

pub async fn get_workflows(State(state): State<AdminState>) -> Json<Vec<WorkflowSummary>> {
    let runtime_state = state.runtime_state.read().await;
    Json(
        runtime_state
            .config
            .workflows
            .iter()
            .map(|workflow| workflow_summary(workflow, &runtime_state.workflow_set))
            .collect(),
    )
}

pub async fn get_workflow(
    Path(id): Path<String>,
    State(state): State<AdminState>,
) -> Result<Json<WorkflowFileConfig>, (StatusCode, String)> {
    let runtime_state = state.runtime_state.read().await;
    workflow_document(&runtime_state.workflow_set, id.as_str())
        .map(Json)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("workflow '{id}' was not found"),
            )
        })
}

pub async fn create_workflow(
    State(state): State<AdminState>,
    Json(input): Json<CreateWorkflowRequest>,
) -> Result<(StatusCode, Json<WorkflowSummary>), (StatusCode, String)> {
    let mut runtime_state = state.runtime_state.write().await;
    let plan = plan_create_workflow(
        &to_application_workflow_entries(&runtime_state.config.workflows),
        runtime_state.config.active_workflow_id.as_deref(),
        &input.id,
        &input.name,
        input.description,
    )
    .map_err(workflow_admin_error)?;
    let mut next_config = runtime_state.config.clone();
    let workflow = from_application_workflow_entry(&plan.workflow);
    next_config.active_workflow_id = plan.next_active_workflow_id;
    next_config.workflows.push(workflow.clone());
    ensure_active_workflow_file(&state.config_path, &runtime_state).map_err(invalid_request)?;

    let workflow_path = resolve_workflow_path(&state.config_path, &next_config, &workflow.file)
        .map_err(invalid_request)?;
    if workflow_path.exists() {
        return Err((
            StatusCode::CONFLICT,
            format!("workflow file '{}' already exists", workflow_path.display()),
        ));
    }

    let document = default_workflow_document();
    save_workflow_file_atomic(&workflow_path, &document).map_err(invalid_request)?;
    let next_runtime_state =
        match runtime_state_from_config(&state.config_path, next_config.clone()) {
            Ok(next_runtime_state) => next_runtime_state,
            Err(error) => {
                let _ = std::fs::remove_file(&workflow_path);
                return Err(invalid_request(error));
            }
        };
    if let Err(error) = save_config_atomic(&state.config_path, &next_config) {
        let _ = std::fs::remove_file(&workflow_path);
        return Err(invalid_request(error));
    }

    let summary = workflow_summary(&workflow, &next_runtime_state.workflow_set);
    *runtime_state = next_runtime_state;
    Ok((StatusCode::CREATED, Json(summary)))
}

pub async fn put_workflow(
    Path(id): Path<String>,
    State(state): State<AdminState>,
    Json(input): Json<WorkflowFileConfig>,
) -> Result<Json<WorkflowFileConfig>, (StatusCode, String)> {
    let mut runtime_state = state.runtime_state.write().await;
    let workflow = require_workflow(
        &to_application_workflow_entries(&runtime_state.config.workflows),
        &id,
    )
    .map_err(workflow_admin_error)
    .map(|workflow| from_application_workflow_entry(&workflow))?;
    let workflow_path =
        resolve_workflow_path(&state.config_path, &runtime_state.config, &workflow.file)
            .map_err(invalid_request)?;
    let previous_document = workflow_document(&runtime_state.workflow_set, id.as_str());

    save_workflow_file_atomic(&workflow_path, &input).map_err(invalid_request)?;
    let next_runtime_state =
        match runtime_state_from_config(&state.config_path, runtime_state.config.clone()) {
            Ok(next_runtime_state) => next_runtime_state,
            Err(error) => {
                if let Some(previous_document) = previous_document {
                    let _ = save_workflow_file_atomic(&workflow_path, &previous_document);
                }
                return Err(invalid_request(error));
            }
        };

    *runtime_state = next_runtime_state;
    Ok(Json(input))
}

pub async fn activate_workflow(
    Path(id): Path<String>,
    State(state): State<AdminState>,
) -> Result<Json<WorkflowSummary>, (StatusCode, String)> {
    let mut runtime_state = state.runtime_state.write().await;
    let workflow = require_workflow(
        &to_application_workflow_entries(&runtime_state.config.workflows),
        &id,
    )
    .map_err(workflow_admin_error)
    .map(|workflow| from_application_workflow_entry(&workflow))?;

    let mut next_config = runtime_state.config.clone();
    next_config.active_workflow_id = Some(id.clone());
    ensure_active_workflow_file(&state.config_path, &runtime_state).map_err(invalid_request)?;
    let next_runtime_state = runtime_state_from_config(&state.config_path, next_config.clone())
        .map_err(invalid_request)?;
    save_config_atomic(&state.config_path, &next_config).map_err(invalid_request)?;

    let summary = workflow_summary(&workflow, &next_runtime_state.workflow_set);
    *runtime_state = next_runtime_state;
    Ok(Json(summary))
}
