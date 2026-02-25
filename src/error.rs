use std::collections::BTreeMap;
use std::io;

use thiserror::Error;

use crate::types::DiagnosticSeverity;

/// Top-level result type for simulation operations.
pub type SimResult<T> = Result<T, SimError>;

#[derive(Debug, Error)]
/// Setup-time validation failures before execution starts.
pub enum SetupError {
    #[error("invalid graph reference `{reference}` in graph `{graph}`")]
    InvalidGraphReference { graph: String, reference: String },
    #[error("invalid parameter `{name}`: {reason}")]
    InvalidParameter { name: String, reason: String },
}

#[derive(Debug, Error)]
/// Runtime failures while executing a simulation run.
pub enum RunError {
    #[error("step overflow: attempted step {attempted} exceeds maximum {max}")]
    StepOverflow { attempted: u64, max: u64 },
    #[error("invalid run configuration `{name}`: {reason}")]
    InvalidRunConfig { name: String, reason: String },
    #[error("event sink error: {message}")]
    EventSink { message: String },
    #[error("run violation [{severity:?}] `{code}`: {message}; evidence={evidence:?}")]
    Violation {
        severity: DiagnosticSeverity,
        code: String,
        message: String,
        evidence: BTreeMap<String, String>,
    },
}

#[derive(Debug, Error)]
/// Assertion evaluation failures.
pub enum AssertionError {
    #[error("expectation mismatch for `{subject}`: expected `{expected}`, got `{actual}`")]
    ExpectationMismatch { subject: String, expected: String, actual: String },
}

#[derive(Debug, Error)]
/// Artifact read/write and serialization failures.
pub enum ArtifactError {
    #[error("artifact I/O error at `{path}`: {source}")]
    Io {
        path: String,
        #[source]
        source: io::Error,
    },
    #[error("artifact serialization error in `{context}`: {source}")]
    Serialization {
        context: String,
        #[source]
        source: serde_json::Error,
    },
}

impl ArtifactError {
    /// Creates an I/O-flavored artifact error with path context.
    pub fn io(path: impl Into<String>, source: io::Error) -> Self {
        Self::Io { path: path.into(), source }
    }

    /// Creates a serialization-flavored artifact error with context label.
    pub fn serialization(context: impl Into<String>, source: serde_json::Error) -> Self {
        Self::Serialization { context: context.into(), source }
    }
}

impl From<io::Error> for ArtifactError {
    fn from(source: io::Error) -> Self {
        Self::io("<unknown>", source)
    }
}

impl From<serde_json::Error> for ArtifactError {
    fn from(source: serde_json::Error) -> Self {
        Self::serialization("<unknown>", source)
    }
}

#[derive(Debug, Error)]
/// Unified error envelope for setup, run, assertion, and artifact layers.
pub enum SimError {
    #[error(transparent)]
    Setup(#[from] SetupError),
    #[error(transparent)]
    Run(#[from] RunError),
    #[error(transparent)]
    Assertion(#[from] AssertionError),
    #[error(transparent)]
    Artifact(#[from] ArtifactError),
}

impl From<io::Error> for SimError {
    fn from(source: io::Error) -> Self {
        Self::Artifact(source.into())
    }
}

impl From<serde_json::Error> for SimError {
    fn from(source: serde_json::Error) -> Self {
        Self::Artifact(source.into())
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use super::{ArtifactError, AssertionError, RunError, SetupError, SimError};
    use crate::types::DiagnosticSeverity;

    #[test]
    fn setup_error_display_text() {
        let err = SetupError::InvalidGraphReference {
            graph: "main".to_string(),
            reference: "node-99".to_string(),
        };

        assert_eq!(err.to_string(), "invalid graph reference `node-99` in graph `main`");
    }

    #[test]
    fn run_error_display_text() {
        let err = RunError::StepOverflow { attempted: 101, max: 100 };

        assert_eq!(err.to_string(), "step overflow: attempted step 101 exceeds maximum 100");
    }

    #[test]
    fn run_error_invalid_config_display_text() {
        let err = RunError::InvalidRunConfig {
            name: "run.max_steps".to_string(),
            reason: "must be greater than 0".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "invalid run configuration `run.max_steps`: must be greater than 0"
        );
    }

    #[test]
    fn run_error_event_sink_display_text() {
        let err = RunError::EventSink { message: "channel closed".to_string() };
        assert_eq!(err.to_string(), "event sink error: channel closed");
    }

    #[test]
    fn run_error_violation_is_typed_and_evidence_rich() {
        let err = RunError::Violation {
            severity: DiagnosticSeverity::Warning,
            code: "RULE-001".to_string(),
            message: "connection blocked".to_string(),
            evidence: std::collections::BTreeMap::from([
                ("edge".to_string(), "edge-a".to_string()),
                ("step".to_string(), "3".to_string()),
            ]),
        };

        assert_eq!(
            err.to_string(),
            "run violation [Warning] `RULE-001`: connection blocked; evidence={\"edge\": \"edge-a\", \"step\": \"3\"}"
        );
    }

    #[test]
    fn assertion_error_display_text() {
        let err = AssertionError::ExpectationMismatch {
            subject: "latency_p95".to_string(),
            expected: "<= 120ms".to_string(),
            actual: "138ms".to_string(),
        };

        assert_eq!(
            err.to_string(),
            "expectation mismatch for `latency_p95`: expected `<= 120ms`, got `138ms`"
        );
    }

    #[test]
    fn artifact_error_display_text() {
        let source = io::Error::new(io::ErrorKind::NotFound, "file missing");
        let err = ArtifactError::io("artifacts/out.json", source);

        assert_eq!(err.to_string(), "artifact I/O error at `artifacts/out.json`: file missing");
    }

    #[test]
    fn artifact_from_io_sets_unknown_path() {
        let err: ArtifactError = io::Error::new(io::ErrorKind::PermissionDenied, "denied").into();

        match err {
            ArtifactError::Io { path, source } => {
                assert_eq!(path, "<unknown>");
                assert_eq!(source.kind(), io::ErrorKind::PermissionDenied);
            }
            other => panic!("expected ArtifactError::Io, got {other:?}"),
        }
    }

    #[test]
    fn artifact_from_serde_sets_unknown_context() {
        let source = serde_json::from_str::<serde_json::Value>("{").expect_err("must fail");
        let err: ArtifactError = source.into();

        match err {
            ArtifactError::Serialization { context, .. } => {
                assert_eq!(context, "<unknown>");
            }
            other => panic!("expected ArtifactError::Serialization, got {other:?}"),
        }
    }

    #[test]
    fn sim_error_from_setup_maps_variant() {
        let sim: SimError = SetupError::InvalidParameter {
            name: "seed".to_string(),
            reason: "must be non-negative".to_string(),
        }
        .into();

        assert!(matches!(sim, SimError::Setup(SetupError::InvalidParameter { .. })));
    }

    #[test]
    fn sim_error_from_io_maps_to_artifact_variant() {
        let sim: SimError = io::Error::new(io::ErrorKind::NotFound, "missing").into();

        match sim {
            SimError::Artifact(ArtifactError::Io { path, source }) => {
                assert_eq!(path, "<unknown>");
                assert_eq!(source.kind(), io::ErrorKind::NotFound);
            }
            other => panic!("expected SimError::Artifact(ArtifactError::Io), got {other:?}"),
        }
    }
}
