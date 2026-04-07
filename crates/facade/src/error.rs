use std::error::Error;
use std::fmt::{self, Display, Formatter};

use application::ApplicationError;
use infrastructure::config_acl::InfrastructureAclError;
use infrastructure::config_store::InfrastructureStoreError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FacadeError {
    InfrastructureAcl(InfrastructureAclError),
    InfrastructureStore(String),
    Application(ApplicationError),
}

impl Display for FacadeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InfrastructureAcl(error) => error.fmt(formatter),
            Self::InfrastructureStore(error) => formatter.write_str(error),
            Self::Application(error) => error.fmt(formatter),
        }
    }
}

impl Error for FacadeError {}

impl From<InfrastructureAclError> for FacadeError {
    fn from(value: InfrastructureAclError) -> Self {
        Self::InfrastructureAcl(value)
    }
}

impl From<InfrastructureStoreError> for FacadeError {
    fn from(value: InfrastructureStoreError) -> Self {
        Self::InfrastructureStore(value.to_string())
    }
}

impl From<ApplicationError> for FacadeError {
    fn from(value: ApplicationError) -> Self {
        Self::Application(value)
    }
}
