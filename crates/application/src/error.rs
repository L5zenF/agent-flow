use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplicationError {
    NoActiveWorkflow,
    SourceLoad(String),
}

impl Display for ApplicationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoActiveWorkflow => formatter.write_str("no active workflow is selected"),
            Self::SourceLoad(message) => formatter.write_str(message),
        }
    }
}

impl Error for ApplicationError {}
