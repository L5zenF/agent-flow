use std::fmt::Display;

use domain::{GatewayCatalog, GatewayConfigSource, WorkflowIndex};

use crate::error::ApplicationError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewaySummary {
    pub provider_count: usize,
    pub model_count: usize,
    pub workflow_count: usize,
    pub active_workflow_id: String,
}

pub fn summarize_gateway_catalog(
    catalog: &GatewayCatalog,
    workflows: &WorkflowIndex,
) -> Result<GatewaySummary, ApplicationError> {
    let active_workflow = workflows.active().ok_or(ApplicationError::NoActiveWorkflow)?;

    Ok(GatewaySummary {
        provider_count: catalog.providers().len(),
        model_count: catalog.models().iter().count(),
        workflow_count: workflows.iter().count(),
        active_workflow_id: active_workflow.id().as_str().to_string(),
    })
}

pub fn summarize_gateway_from_source<S, E>(source: &S) -> Result<GatewaySummary, ApplicationError>
where
    S: GatewayConfigSource<Error = E>,
    E: Display,
{
    let (catalog, workflows) = source
        .load_gateway_state()
        .map_err(|error| ApplicationError::SourceLoad(error.to_string()))?;
    summarize_gateway_catalog(&catalog, &workflows)
}

#[cfg(test)]
mod tests {
    use domain::{
        GatewayCatalog, Model, ModelCatalog, ModelId, Provider, ProviderCatalog, ProviderId,
        RouteId, Workflow, WorkflowId, WorkflowIndex,
    };

    use crate::{ApplicationError, GatewaySummary, summarize_gateway_catalog, summarize_gateway_from_source};

    #[test]
    fn summarizes_catalog_with_active_workflow() {
        let gateway_catalog = GatewayCatalog::new(
            ProviderCatalog::new(vec![
                Provider::new(ProviderId::new("openai").expect("valid id"), "OpenAI")
                    .expect("valid provider"),
            ])
            .expect("valid provider catalog"),
            ModelCatalog::new(vec![
                Model::new(
                    ModelId::new("gpt-4o").expect("valid id"),
                    ProviderId::new("openai").expect("valid provider id"),
                    "GPT-4o",
                )
                .expect("valid model"),
            ])
            .expect("valid model catalog"),
        )
        .expect("valid gateway catalog");
        let workflows = WorkflowIndex::new(
            vec![Workflow::new(
                WorkflowId::new("chat-routing").expect("valid id"),
                vec![RouteId::new("route-a").expect("valid route id")],
            )
            .expect("valid workflow")],
            Some(WorkflowId::new("chat-routing").expect("valid id")),
        )
        .expect("valid workflow index");

        let summary = summarize_gateway_catalog(&gateway_catalog, &workflows).expect("summary");

        assert_eq!(summary.provider_count, 1);
        assert_eq!(summary.model_count, 1);
        assert_eq!(summary.workflow_count, 1);
        assert_eq!(summary.active_workflow_id, "chat-routing");
    }

    #[test]
    fn rejects_summary_without_active_workflow() {
        let gateway_catalog = GatewayCatalog::new(
            ProviderCatalog::new(vec![
                Provider::new(ProviderId::new("openai").expect("valid id"), "OpenAI")
                    .expect("valid provider"),
            ])
            .expect("valid provider catalog"),
            ModelCatalog::new(vec![
                Model::new(
                    ModelId::new("gpt-4o").expect("valid id"),
                    ProviderId::new("openai").expect("valid provider id"),
                    "GPT-4o",
                )
                .expect("valid model"),
            ])
            .expect("valid model catalog"),
        )
        .expect("valid gateway catalog");
        let workflows = WorkflowIndex::new(
            vec![Workflow::new(
                WorkflowId::new("chat-routing").expect("valid id"),
                vec![RouteId::new("route-a").expect("valid route id")],
            )
            .expect("valid workflow")],
            None,
        )
        .expect("valid workflow index");

        let error = summarize_gateway_catalog(&gateway_catalog, &workflows)
            .expect_err("active workflow is required for summary");

        assert_eq!(error, ApplicationError::NoActiveWorkflow);
    }

    struct InMemorySource {
        catalog: GatewayCatalog,
        workflows: WorkflowIndex,
    }

    impl domain::GatewayConfigSource for InMemorySource {
        type Error = std::convert::Infallible;

        fn load_gateway_state(&self) -> Result<(GatewayCatalog, WorkflowIndex), Self::Error> {
            Ok((self.catalog.clone(), self.workflows.clone()))
        }
    }

    #[test]
    fn summarizes_gateway_from_source_port() {
        let catalog = GatewayCatalog::new(
            ProviderCatalog::new(vec![
                Provider::new(ProviderId::new("openai").expect("valid id"), "OpenAI")
                    .expect("valid provider"),
            ])
            .expect("valid provider catalog"),
            ModelCatalog::new(vec![
                Model::new(
                    ModelId::new("gpt-4o").expect("valid id"),
                    ProviderId::new("openai").expect("valid provider id"),
                    "GPT-4o",
                )
                .expect("valid model"),
            ])
            .expect("valid model catalog"),
        )
        .expect("valid gateway catalog");
        let workflows = WorkflowIndex::new(
            vec![Workflow::new(
                WorkflowId::new("chat-routing").expect("valid id"),
                vec![RouteId::new("route-a").expect("valid route id")],
            )
            .expect("valid workflow")],
            Some(WorkflowId::new("chat-routing").expect("valid id")),
        )
        .expect("valid workflow index");
        let source = InMemorySource { catalog, workflows };

        let summary: GatewaySummary =
            summarize_gateway_from_source(&source).expect("summary from source should succeed");

        assert_eq!(summary.provider_count, 1);
        assert_eq!(summary.active_workflow_id, "chat-routing");
    }
}
