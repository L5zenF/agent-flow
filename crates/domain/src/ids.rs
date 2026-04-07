use std::fmt::{self, Display, Formatter};

use crate::error::DomainError;

macro_rules! typed_id {
    ($name:ident, $error_variant:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            /// Creates an identifier using only minimal normalization: surrounding
            /// whitespace is trimmed, but casing and internal characters are
            /// preserved for upstream mapping layers to interpret explicitly.
            pub fn new(value: impl Into<String>) -> Result<Self, DomainError> {
                let value = value.into().trim().to_string();
                if value.is_empty() {
                    return Err(DomainError::$error_variant);
                }

                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl Display for $name {
            fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }
    };
}

typed_id!(ProviderId, BlankProviderId);
typed_id!(ModelId, BlankModelId);
typed_id!(RouteId, BlankRouteId);
typed_id!(WorkflowId, BlankWorkflowId);
