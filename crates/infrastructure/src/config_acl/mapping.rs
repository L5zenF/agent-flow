use domain::{
    GatewayCatalog, Model, ModelCatalog, ModelId, Provider, ProviderCatalog, ProviderId, RouteId,
    Workflow, WorkflowId, WorkflowIndex,
};

use super::{InfrastructureAclError, RawModel, RawProvider, RawWorkflowIndex};

pub fn map_gateway_catalog(
    raw_providers: &[RawProvider],
    raw_models: &[RawModel],
) -> Result<GatewayCatalog, InfrastructureAclError> {
    let providers = raw_providers
        .iter()
        .map(|provider| {
            Ok(Provider::new(
                ProviderId::new(provider.id.clone())?,
                provider.name.clone(),
            )?)
        })
        .collect::<Result<Vec<_>, InfrastructureAclError>>()?;
    let models = raw_models
        .iter()
        .map(|model| {
            Ok(Model::new(
                ModelId::new(model.id.clone())?,
                ProviderId::new(model.provider_id.clone())?,
                model.name.clone(),
            )?)
        })
        .collect::<Result<Vec<_>, InfrastructureAclError>>()?;

    let provider_catalog = ProviderCatalog::new(providers)?;
    let model_catalog = ModelCatalog::new(models)?;

    Ok(GatewayCatalog::new(provider_catalog, model_catalog)?)
}

pub fn map_workflow_index(
    raw_workflow_index: &RawWorkflowIndex,
) -> Result<WorkflowIndex, InfrastructureAclError> {
    let workflows = raw_workflow_index
        .workflows
        .iter()
        .map(|workflow| {
            let workflow_id = WorkflowId::new(workflow.id.clone())?;
            let route_id = RouteId::new(workflow_id.as_str().to_string())?;
            Ok(Workflow::new(workflow_id, vec![route_id])?)
        })
        .collect::<Result<Vec<_>, InfrastructureAclError>>()?;
    let active_workflow_id = raw_workflow_index
        .active_workflow_id
        .as_ref()
        .map(WorkflowId::new)
        .transpose()?;

    Ok(WorkflowIndex::new(workflows, active_workflow_id)?)
}
