use std::error::Error;
use std::fmt::{self, Display, Formatter};

use domain::DomainError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InfrastructureAclError {
    Domain(DomainError),
    TomlDeserialize(String),
    Validation(String),
}

impl Display for InfrastructureAclError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Domain(error) => error.fmt(formatter),
            Self::TomlDeserialize(error) => formatter.write_str(error),
            Self::Validation(error) => formatter.write_str(error),
        }
    }
}

impl Error for InfrastructureAclError {}

impl From<DomainError> for InfrastructureAclError {
    fn from(value: DomainError) -> Self {
        Self::Domain(value)
    }
}
