use crate::{GatewayCatalog, WorkflowIndex};

pub trait GatewayConfigSource {
    type Error;

    fn load_gateway_state(&self) -> Result<(GatewayCatalog, WorkflowIndex), Self::Error>;
}
