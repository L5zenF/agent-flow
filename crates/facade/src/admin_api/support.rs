use axum::http::StatusCode;

use application::{WorkflowAdminError, WorkflowEntryInput};

use crate::admin_api::types::WorkflowSummary;
use crate::config::{
    resolve_workflow_path, LoadedWorkflowSet, RuleGraphConfig, RuntimeState, WorkflowFileConfig,
    WorkflowIndexEntry,
};
use infrastructure::plugin_registry::{
    ManifestCapability, ManifestCategory, ManifestIcon, ManifestTone,
};

pub fn manifest_capability_name(capability: &ManifestCapability) -> &'static str {
    match capability {
        ManifestCapability::Log => "log",
        ManifestCapability::Fs => "fs",
        ManifestCapability::Network => "network",
    }
}

pub fn manifest_icon_name(icon: &ManifestIcon) -> &'static str {
    match icon {
        ManifestIcon::Puzzle => "puzzle",
        ManifestIcon::Split => "split",
        ManifestIcon::Route => "route",
        ManifestIcon::Wand => "wand",
        ManifestIcon::Shield => "shield",
        ManifestIcon::Code => "code",
        ManifestIcon::Filter => "filter",
        ManifestIcon::Database => "database",
        ManifestIcon::FileText => "file_text",
    }
}

pub fn manifest_category_name(category: &ManifestCategory) -> &'static str {
    match category {
        ManifestCategory::Control => "control",
        ManifestCategory::Transform => "transform",
        ManifestCategory::Routing => "routing",
        ManifestCategory::Policy => "policy",
        ManifestCategory::Utility => "utility",
    }
}

pub fn manifest_tone_name(tone: &ManifestTone) -> &'static str {
    match tone {
        ManifestTone::Slate => "slate",
        ManifestTone::Blue => "blue",
        ManifestTone::Sky => "sky",
        ManifestTone::Teal => "teal",
        ManifestTone::Emerald => "emerald",
        ManifestTone::Amber => "amber",
        ManifestTone::Rose => "rose",
        ManifestTone::Violet => "violet",
    }
}

pub fn workflow_document(workflows: &LoadedWorkflowSet, id: &str) -> Option<WorkflowFileConfig> {
    workflows.by_id.get(id).cloned().or_else(|| {
        (workflows.active_workflow_id.as_deref() == Some(id))
            .then(|| workflows.active_graph().cloned())
            .flatten()
            .map(|workflow| WorkflowFileConfig { workflow })
    })
}

pub fn to_application_workflow_entries(workflows: &[WorkflowIndexEntry]) -> Vec<WorkflowEntryInput> {
    workflows
        .iter()
        .map(|workflow| WorkflowEntryInput {
            id: workflow.id.clone(),
            name: workflow.name.clone(),
            file: workflow.file.clone(),
            description: workflow.description.clone(),
        })
        .collect()
}

pub fn from_application_workflow_entry(workflow: &WorkflowEntryInput) -> WorkflowIndexEntry {
    WorkflowIndexEntry {
        id: workflow.id.clone(),
        name: workflow.name.clone(),
        file: workflow.file.clone(),
        description: workflow.description.clone(),
    }
}

pub fn workflow_summary(
    workflow: &WorkflowIndexEntry,
    workflows: &LoadedWorkflowSet,
) -> WorkflowSummary {
    let graph = workflow_document(workflows, workflow.id.as_str()).map(|document| document.workflow);
    WorkflowSummary {
        id: workflow.id.clone(),
        name: workflow.name.clone(),
        description: workflow.description.clone(),
        file: workflow.file.clone(),
        is_active: workflows.active_workflow_id.as_deref() == Some(workflow.id.as_str()),
        node_count: graph.as_ref().map_or(0, |graph| graph.nodes.len()),
        edge_count: graph.as_ref().map_or(0, |graph| graph.edges.len()),
    }
}

pub fn ensure_active_workflow_file(
    config_path: &std::path::Path,
    runtime_state: &RuntimeState,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(active_workflow_id) = runtime_state.config.active_workflow_id.as_deref() else {
        return Ok(());
    };
    if runtime_state.workflow_set.by_id.contains_key(active_workflow_id) {
        return Ok(());
    }

    let Some(active_graph) = runtime_state.workflow_set.active_graph() else {
        return Ok(());
    };
    let Some(workflow) = runtime_state
        .config
        .workflows
        .iter()
        .find(|workflow| workflow.id == active_workflow_id)
    else {
        return Ok(());
    };

    let workflow_path = resolve_workflow_path(config_path, &runtime_state.config, &workflow.file)?;
    crate::config::save_workflow_file_atomic(
        &workflow_path,
        &WorkflowFileConfig {
            workflow: active_graph.clone(),
        },
    )
}

pub fn default_workflow_document() -> WorkflowFileConfig {
    WorkflowFileConfig {
        workflow: RuleGraphConfig {
            version: 1,
            start_node_id: "start".to_string(),
            nodes: vec![crate::config::RuleGraphNode {
                id: "start".to_string(),
                node_type: crate::config::RuleGraphNodeType::Start,
                position: crate::config::GraphPosition { x: 0.0, y: 0.0 },
                note: None,
                condition: None,
                route_provider: None,
                select_model: None,
                rewrite_path: None,
                set_context: None,
                router: None,
                log: None,
                set_header: None,
                remove_header: None,
                copy_header: None,
                set_header_if_absent: None,
                note_node: None,
                wasm_plugin: None,
                match_node: None,
                code_runner: None,
            }],
            edges: Vec::new(),
        },
    }
}

pub fn workflow_admin_error(error: WorkflowAdminError) -> (StatusCode, String) {
    let status = match error {
        WorkflowAdminError::DuplicateWorkflowId(_) => StatusCode::CONFLICT,
        WorkflowAdminError::WorkflowNotFound(_) => StatusCode::NOT_FOUND,
        WorkflowAdminError::EmptyWorkflowId | WorkflowAdminError::EmptyWorkflowName => {
            StatusCode::BAD_REQUEST
        }
    };
    (status, error.message())
}

pub fn invalid_request(error: impl ToString) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, error.to_string())
}
