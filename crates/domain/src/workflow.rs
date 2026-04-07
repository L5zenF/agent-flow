use std::collections::{BTreeMap, BTreeSet};

use crate::error::DomainError;
use crate::ids::{RouteId, WorkflowId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workflow {
    id: WorkflowId,
    routes: Vec<RouteId>,
}

impl Workflow {
    pub fn new(id: WorkflowId, routes: Vec<RouteId>) -> Result<Self, DomainError> {
        if routes.is_empty() {
            return Err(DomainError::EmptyWorkflowRoutes { workflow_id: id });
        }

        let mut seen = BTreeSet::new();
        for route_id in &routes {
            if !seen.insert(route_id.clone()) {
                return Err(DomainError::DuplicateRouteId {
                    workflow_id: id.clone(),
                    route_id: route_id.clone(),
                });
            }
        }

        Ok(Self { id, routes })
    }

    pub fn id(&self) -> &WorkflowId {
        &self.id
    }

    pub fn routes(&self) -> &[RouteId] {
        &self.routes
    }

    pub fn contains_route(&self, route_id: &RouteId) -> bool {
        self.routes.iter().any(|route| route == route_id)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkflowIndex {
    workflows: BTreeMap<WorkflowId, Workflow>,
    active_workflow_id: Option<WorkflowId>,
}

impl WorkflowIndex {
    pub fn new(
        workflows: Vec<Workflow>,
        active_workflow_id: Option<WorkflowId>,
    ) -> Result<Self, DomainError> {
        let mut indexed = BTreeMap::new();

        for workflow in workflows {
            let workflow_id = workflow.id().clone();
            if indexed.insert(workflow_id.clone(), workflow).is_some() {
                return Err(DomainError::DuplicateWorkflowId { workflow_id });
            }
        }

        let active_workflow_id = match active_workflow_id {
            None => None,
            Some(workflow_id) if indexed.is_empty() => {
                return Err(DomainError::ActiveWorkflowDefinedWithoutWorkflows { workflow_id });
            }
            Some(workflow_id) => {
                if !indexed.contains_key(&workflow_id) {
                    return Err(DomainError::ActiveWorkflowNotFound { workflow_id });
                }
                Some(workflow_id)
            }
        };

        Ok(Self {
            workflows: indexed,
            active_workflow_id,
        })
    }

    pub fn get(&self, workflow_id: &WorkflowId) -> Option<&Workflow> {
        self.workflows.get(workflow_id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Workflow> {
        self.workflows.values()
    }

    pub fn active(&self) -> Option<&Workflow> {
        self.active_workflow_id
            .as_ref()
            .and_then(|workflow_id| self.workflows.get(workflow_id))
    }

    pub fn active_id(&self) -> Option<&WorkflowId> {
        self.active_workflow_id.as_ref()
    }

    pub fn activate(&mut self, workflow_id: &WorkflowId) -> Result<(), DomainError> {
        if !self.workflows.contains_key(workflow_id) {
            return Err(DomainError::ActiveWorkflowNotFound {
                workflow_id: workflow_id.clone(),
            });
        }

        self.active_workflow_id = Some(workflow_id.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Workflow, WorkflowIndex};
    use crate::{DomainError, RouteId, WorkflowId};

    #[test]
    fn workflow_rejects_duplicate_routes() {
        let result = Workflow::new(
            WorkflowId::new("primary").unwrap(),
            vec![RouteId::new("chat").unwrap(), RouteId::new("chat").unwrap()],
        );

        assert_eq!(
            result.unwrap_err(),
            DomainError::DuplicateRouteId {
                workflow_id: WorkflowId::new("primary").unwrap(),
                route_id: RouteId::new("chat").unwrap(),
            }
        );
    }

    #[test]
    fn empty_index_has_no_active_workflow() {
        let index = WorkflowIndex::new(Vec::new(), None).unwrap();

        assert!(index.active().is_none());
        assert!(index.active_id().is_none());
        assert_eq!(index.iter().count(), 0);
    }
}
