use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::gateway_config::model::RuleGraphConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowIndexEntry {
    pub id: String,
    pub name: String,
    pub file: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowFileConfig {
    pub workflow: RuleGraphConfig,
}

#[derive(Debug, Clone, Default)]
pub struct LoadedWorkflowSet {
    pub summaries: Vec<WorkflowIndexEntry>,
    pub by_id: BTreeMap<String, WorkflowFileConfig>,
    pub active_workflow_id: Option<String>,
    pub(crate) legacy_rule_graph: Option<RuleGraphConfig>,
}

impl LoadedWorkflowSet {
    pub fn active_graph(&self) -> Option<&RuleGraphConfig> {
        self.active_workflow_id
            .as_deref()
            .and_then(|workflow_id| self.by_id.get(workflow_id))
            .map(|workflow| &workflow.workflow)
            .or(self.legacy_rule_graph.as_ref())
    }
}
