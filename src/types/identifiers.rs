use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
/// Validation errors for strongly-typed identifier wrappers.
pub enum IdentifierError {
    #[error("{kind} cannot be empty")]
    Empty { kind: &'static str },
    #[error("{kind} cannot contain control characters")]
    ContainsControl { kind: &'static str },
}

macro_rules! define_identifier {
    ($name:ident, $kind:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        #[doc = concat!("Validated ", $kind, " wrapper used in persisted/session-facing APIs.")]
        pub struct $name(String);

        impl $name {
            #[doc = concat!("Creates a validated ", $kind, ".")]
            pub fn new(value: impl Into<String>) -> Result<Self, IdentifierError> {
                let value = value.into();
                if value.trim().is_empty() {
                    return Err(IdentifierError::Empty { kind: $kind });
                }
                if value.chars().any(char::is_control) {
                    return Err(IdentifierError::ContainsControl { kind: $kind });
                }
                Ok(Self(value))
            }

            #[doc = concat!("Creates a fixture ", $kind, " and panics if invalid.")]
            pub fn fixture(value: impl Into<String>) -> Self {
                Self::new(value).expect(concat!("invalid ", $kind, " fixture identifier"))
            }

            #[doc = concat!("Returns the ", $kind, " as `&str`.")]
            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl TryFrom<&str> for $name {
            type Error = IdentifierError;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl TryFrom<String> for $name {
            type Error = IdentifierError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

define_identifier!(ScenarioId, "scenario id");
define_identifier!(NodeId, "node id");
define_identifier!(EdgeId, "edge id");
define_identifier!(MetricKey, "metric key");
