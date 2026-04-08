use std::collections::HashSet;

use super::{InfrastructureAclError, RawGatewayConfig, util::validate_relative_path};

pub(super) fn validate_workflow_index_subset(
    raw: &RawGatewayConfig,
) -> Result<(), InfrastructureAclError> {
    WorkflowIndexAcl::new(raw).validate()
}

struct WorkflowIndexAcl<'a> {
    raw: &'a RawGatewayConfig,
}

impl<'a> WorkflowIndexAcl<'a> {
    fn new(raw: &'a RawGatewayConfig) -> Self {
        Self { raw }
    }

    fn validate(&self) -> Result<(), InfrastructureAclError> {
        self.validate_workflows_dir()?;
        self.validate_workflow_files()?;
        self.validate_active_workflow()?;
        Ok(())
    }

    fn validate_workflows_dir(&self) -> Result<(), InfrastructureAclError> {
        if let Some(workflows_dir) = self.raw.workflows_dir.as_deref() {
            validate_relative_path("workflows_dir", workflows_dir)?;
        }
        Ok(())
    }

    fn validate_workflow_files(&self) -> Result<(), InfrastructureAclError> {
        let mut workflow_files = HashSet::new();
        for workflow in &self.raw.workflows {
            let file = workflow.file.as_deref().ok_or_else(|| {
                InfrastructureAclError::Validation(format!(
                    "workflow '{}' file cannot be empty",
                    workflow.id
                ))
            })?;
            if file.trim().is_empty() {
                return Err(InfrastructureAclError::Validation(format!(
                    "workflow '{}' file cannot be empty",
                    workflow.id
                )));
            }
            if !workflow_files.insert(file.to_string()) {
                return Err(InfrastructureAclError::Validation(format!(
                    "duplicate workflow file '{}'",
                    file
                )));
            }
            validate_relative_path(&format!("workflow '{}' file", workflow.id), file)?;
        }
        Ok(())
    }

    fn validate_active_workflow(&self) -> Result<(), InfrastructureAclError> {
        if self.raw.workflows.is_empty() {
            return Ok(());
        }

        let active_id = self.raw.active_workflow_id.as_deref().ok_or_else(|| {
            InfrastructureAclError::Validation(
                "active_workflow_id must be set when workflows are present".to_string(),
            )
        })?;
        if !self
            .raw
            .workflows
            .iter()
            .any(|workflow| workflow.id == active_id)
        {
            return Err(InfrastructureAclError::Validation(format!(
                "active_workflow_id '{}' does not reference an indexed workflow",
                active_id
            )));
        }
        Ok(())
    }
}
