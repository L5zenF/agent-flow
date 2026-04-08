use std::collections::HashSet;

use crate::{ApplicationError, WorkflowValidationInput};

use super::shared::{unique_ids, validate_workflow_relative_path};

pub(super) struct WorkflowIndexValidator<'a> {
    workflows_dir: Option<&'a str>,
    active_workflow_id: Option<&'a str>,
    workflows: &'a [WorkflowValidationInput],
    workflow_ids: HashSet<String>,
}

impl<'a> WorkflowIndexValidator<'a> {
    pub(super) fn new(
        workflows_dir: Option<&'a str>,
        active_workflow_id: Option<&'a str>,
        workflows: &'a [WorkflowValidationInput],
    ) -> Result<Self, ApplicationError> {
        let workflow_ids = unique_ids(
            workflows.iter().map(|workflow| workflow.id.as_str()),
            "workflow",
        )?;
        Ok(Self {
            workflows_dir,
            active_workflow_id,
            workflows,
            workflow_ids,
        })
    }

    pub(super) fn validate(&self) -> Result<(), ApplicationError> {
        self.validate_workflows_dir()?;
        self.validate_workflow_files()?;
        self.validate_active_workflow()?;
        Ok(())
    }

    fn validate_workflows_dir(&self) -> Result<(), ApplicationError> {
        if let Some(workflows_dir) = self.workflows_dir {
            validate_workflow_relative_path("workflows_dir", workflows_dir)?;
        }
        Ok(())
    }

    fn validate_workflow_files(&self) -> Result<(), ApplicationError> {
        let mut workflow_files = HashSet::new();
        for workflow in self.workflows {
            if !workflow_files.insert(workflow.file.as_str()) {
                return Err(ApplicationError::Validation(format!(
                    "duplicate workflow file '{}'",
                    workflow.file
                )));
            }
        }

        for workflow in self.workflows {
            if workflow.file.trim().is_empty() {
                return Err(ApplicationError::Validation(format!(
                    "workflow '{}' file cannot be empty",
                    workflow.id
                )));
            }
            validate_workflow_relative_path(
                &format!("workflow '{}' file", workflow.id),
                &workflow.file,
            )?;
        }
        Ok(())
    }

    fn validate_active_workflow(&self) -> Result<(), ApplicationError> {
        if self.workflows.is_empty() {
            return Ok(());
        }

        let Some(active_workflow_id) = self.active_workflow_id else {
            return Err(ApplicationError::Validation(
                "active_workflow_id must be set when workflows are present".to_string(),
            ));
        };
        if !self.workflow_ids.contains(active_workflow_id) {
            return Err(ApplicationError::Validation(format!(
                "active_workflow_id '{}' does not reference an indexed workflow",
                active_workflow_id
            )));
        }
        Ok(())
    }
}
