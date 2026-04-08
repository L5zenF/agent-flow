use crate::gateway_config::model::{GatewayConfig, LoadedWorkflowSet};

#[derive(Debug, Clone)]
pub struct RuntimeState {
    pub config: GatewayConfig,
    pub workflow_set: LoadedWorkflowSet,
}
