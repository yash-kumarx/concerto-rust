//! Error types for `concerto-core`.
//!
//! The error model deliberately separates *hard* model/runtime errors — where
//! the model itself cannot be used and validation cannot continue — from the
//! ordinary validation failures that callers surface to end users. Only the
//! former are represented here as [`ConcertoError`]; instance validation
//! failures are collected into a structured result by the validation layer.

use thiserror::Error;

/// Convenience `Result` alias used throughout `concerto-core`.
pub type Result<T> = std::result::Result<T, ConcertoError>;

/// A hard error raised while loading models or resolving types.
#[derive(Debug, Error)]
pub enum ConcertoError {
    /// A fully-qualified type could not be resolved in any loaded model.
    #[error("type not found: {type_name}")]
    TypeNotFound {
        /// The fully-qualified (or short) name that failed to resolve.
        type_name: String,
    },

    /// A namespace was referenced but has not been loaded.
    #[error("namespace not found: {namespace}")]
    NamespaceNotFound {
        /// The namespace that could not be located.
        namespace: String,
    },

    /// The model JSON is structurally invalid or violates a model rule.
    #[error("illegal model: {message}")]
    IllegalModel {
        /// Human-readable description of the problem.
        message: String,
        /// The model file the problem originated from, if known.
        file_name: Option<String>,
        /// The source location of the problem, if known.
        location: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_not_found_displays_name() {
        let err = ConcertoError::TypeNotFound {
            type_name: "org.acme@1.0.0.Foo".into(),
        };
        assert!(err.to_string().contains("org.acme@1.0.0.Foo"));
    }

    #[test]
    fn illegal_model_displays_message() {
        let err = ConcertoError::IllegalModel {
            message: "missing 'namespace'".into(),
            file_name: Some("model.json".into()),
            location: None,
        };
        assert!(err.to_string().contains("missing 'namespace'"));
    }
}
