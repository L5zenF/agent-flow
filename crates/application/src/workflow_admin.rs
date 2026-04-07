#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowEntryInput {
    pub id: String,
    pub name: String,
    pub file: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowCreatePlan {
    pub workflow: WorkflowEntryInput,
    pub next_active_workflow_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowAdminError {
    EmptyWorkflowId,
    EmptyWorkflowName,
    DuplicateWorkflowId(String),
    WorkflowNotFound(String),
}

impl WorkflowAdminError {
    pub fn message(&self) -> String {
        match self {
            Self::EmptyWorkflowId => "workflow id cannot be empty".to_string(),
            Self::EmptyWorkflowName => "workflow name cannot be empty".to_string(),
            Self::DuplicateWorkflowId(id) => format!("workflow '{id}' already exists"),
            Self::WorkflowNotFound(id) => format!("workflow '{id}' was not found"),
        }
    }
}

pub fn plan_create_workflow(
    existing: &[WorkflowEntryInput],
    current_active_workflow_id: Option<&str>,
    requested_id: &str,
    requested_name: &str,
    requested_description: Option<String>,
) -> Result<WorkflowCreatePlan, WorkflowAdminError> {
    let id = requested_id.trim();
    if id.is_empty() {
        return Err(WorkflowAdminError::EmptyWorkflowId);
    }
    let name = requested_name.trim();
    if name.is_empty() {
        return Err(WorkflowAdminError::EmptyWorkflowName);
    }
    if existing.iter().any(|workflow| workflow.id == id) {
        return Err(WorkflowAdminError::DuplicateWorkflowId(id.to_string()));
    }

    Ok(WorkflowCreatePlan {
        workflow: WorkflowEntryInput {
            id: id.to_string(),
            name: name.to_string(),
            file: format!("{id}.toml"),
            description: requested_description.and_then(normalize_optional_text),
        },
        next_active_workflow_id: current_active_workflow_id
            .map(str::to_string)
            .or_else(|| Some(id.to_string())),
    })
}

pub fn require_workflow(
    existing: &[WorkflowEntryInput],
    workflow_id: &str,
) -> Result<WorkflowEntryInput, WorkflowAdminError> {
    existing
        .iter()
        .find(|workflow| workflow.id == workflow_id)
        .cloned()
        .ok_or_else(|| WorkflowAdminError::WorkflowNotFound(workflow_id.to_string()))
}

fn normalize_optional_text(value: String) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use crate::{WorkflowAdminError, WorkflowEntryInput, plan_create_workflow, require_workflow};

    #[test]
    fn plans_create_with_trimmed_fields() {
        let plan = plan_create_workflow(&[], None, " demo ", " Demo ", Some(" desc ".to_string()))
            .expect("create plan should be valid");
        assert_eq!(plan.workflow.id, "demo");
        assert_eq!(plan.workflow.name, "Demo");
        assert_eq!(plan.workflow.file, "demo.toml");
        assert_eq!(plan.workflow.description.as_deref(), Some("desc"));
        assert_eq!(plan.next_active_workflow_id.as_deref(), Some("demo"));
    }

    #[test]
    fn rejects_duplicate_workflow_ids() {
        let existing = vec![WorkflowEntryInput {
            id: "demo".to_string(),
            name: "Demo".to_string(),
            file: "demo.toml".to_string(),
            description: None,
        }];
        let error = plan_create_workflow(&existing, Some("demo"), "demo", "Demo", None)
            .expect_err("duplicate id should fail");
        assert_eq!(
            error,
            WorkflowAdminError::DuplicateWorkflowId("demo".to_string())
        );
    }

    #[test]
    fn requires_existing_workflow() {
        let existing = vec![WorkflowEntryInput {
            id: "demo".to_string(),
            name: "Demo".to_string(),
            file: "demo.toml".to_string(),
            description: None,
        }];
        let workflow = require_workflow(&existing, "demo").expect("workflow should exist");
        assert_eq!(workflow.id, "demo");
    }
}
