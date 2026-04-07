use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplicationError {
    ConfigAdmin(String),
    NoActiveWorkflow,
    Policy(String),
    SourceLoad(String),
    Validation(String),
}

impl Display for ApplicationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfigAdmin(message) => formatter.write_str(message),
            Self::NoActiveWorkflow => formatter.write_str("no active workflow is selected"),
            Self::Policy(message) => formatter.write_str(message),
            Self::SourceLoad(message) => formatter.write_str(message),
            Self::Validation(message) => formatter.write_str(message),
        }
    }
}

impl Error for ApplicationError {}
