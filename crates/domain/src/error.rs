use std::error::Error;
use std::fmt::{self, Display, Formatter};

use crate::ids::{ModelId, ProviderId, RouteId, WorkflowId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    BlankProviderId,
    BlankModelId,
    BlankRouteId,
    BlankWorkflowId,
    BlankProviderName,
    BlankModelName,
    EmptyWorkflowRoutes {
        workflow_id: WorkflowId,
    },
    DuplicateRouteId {
        workflow_id: WorkflowId,
        route_id: RouteId,
    },
    DuplicateProviderId {
        provider_id: ProviderId,
    },
    DuplicateModelId {
        model_id: ModelId,
    },
    DuplicateWorkflowId {
        workflow_id: WorkflowId,
    },
    UnknownProviderReference {
        model_id: ModelId,
        provider_id: ProviderId,
    },
    ActiveWorkflowNotFound {
        workflow_id: WorkflowId,
    },
    ActiveWorkflowDefinedWithoutWorkflows {
        workflow_id: WorkflowId,
    },
}

impl Display for DomainError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::BlankProviderId => formatter.write_str("provider identifier cannot be blank"),
            Self::BlankModelId => formatter.write_str("model identifier cannot be blank"),
            Self::BlankRouteId => formatter.write_str("route identifier cannot be blank"),
            Self::BlankWorkflowId => formatter.write_str("workflow identifier cannot be blank"),
            Self::BlankProviderName => formatter.write_str("provider name cannot be blank"),
            Self::BlankModelName => formatter.write_str("model name cannot be blank"),
            Self::EmptyWorkflowRoutes { workflow_id } => {
                write!(
                    formatter,
                    "workflow `{workflow_id}` must contain at least one route"
                )
            }
            Self::DuplicateRouteId {
                workflow_id,
                route_id,
            } => write!(
                formatter,
                "workflow `{workflow_id}` contains duplicate route `{route_id}`"
            ),
            Self::DuplicateProviderId { provider_id } => {
                write!(
                    formatter,
                    "provider `{provider_id}` is defined more than once"
                )
            }
            Self::DuplicateModelId { model_id } => {
                write!(formatter, "model `{model_id}` is defined more than once")
            }
            Self::DuplicateWorkflowId { workflow_id } => {
                write!(
                    formatter,
                    "workflow `{workflow_id}` is defined more than once"
                )
            }
            Self::UnknownProviderReference {
                model_id,
                provider_id,
            } => write!(
                formatter,
                "model `{model_id}` references unknown provider `{provider_id}`"
            ),
            Self::ActiveWorkflowNotFound { workflow_id } => {
                write!(formatter, "active workflow `{workflow_id}` does not exist")
            }
            Self::ActiveWorkflowDefinedWithoutWorkflows { workflow_id } => write!(
                formatter,
                "active workflow `{workflow_id}` cannot be set when no workflows exist"
            ),
        }
    }
}

impl Error for DomainError {}
