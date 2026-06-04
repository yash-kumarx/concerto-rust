//! Small, dependency-free helpers for working with Concerto names and
//! namespaces.
//!
//! Concerto identifies declarations by a fully-qualified name (FQN) of the
//! form `namespace.ShortName`, where the namespace may itself carry a
//! `@version` suffix (e.g. `org.example@1.0.0.Person`). These helpers split
//! and rebuild those names; type resolution proper lives in the model layer.

use crate::error::{ConcertoError, Result};

/// The six primitive Concerto types. Anything else is a declared type.
const PRIMITIVE_TYPES: &[&str] = &["Boolean", "String", "DateTime", "Double", "Integer", "Long"];

/// Returns the short type name: everything after the final `.`.
///
/// ```
/// # use concerto_core::model_util::short_name;
/// assert_eq!(short_name("org.example@1.0.0.Person"), "Person");
/// assert_eq!(short_name("Person"), "Person");
/// ```
pub fn short_name(fqn: &str) -> &str {
    match fqn.rfind('.') {
        Some(i) => &fqn[i + 1..],
        None => fqn,
    }
}

/// Returns the namespace: everything before the final `.`, or `""` when the
/// name is unqualified.
///
/// ```
/// # use concerto_core::model_util::namespace_of;
/// assert_eq!(namespace_of("org.example@1.0.0.Person"), "org.example@1.0.0");
/// assert_eq!(namespace_of("Person"), "");
/// ```
pub fn namespace_of(fqn: &str) -> &str {
    match fqn.rfind('.') {
        Some(i) => &fqn[..i],
        None => "",
    }
}

/// Joins a namespace and a short name into a fully-qualified name. An empty
/// namespace yields the bare short name (used for primitives).
///
/// ```
/// # use concerto_core::model_util::qualify;
/// assert_eq!(qualify("org.example@1.0.0", "Person"), "org.example@1.0.0.Person");
/// assert_eq!(qualify("", "String"), "String");
/// ```
pub fn qualify(namespace: &str, short: &str) -> String {
    if namespace.is_empty() {
        short.to_string()
    } else {
        format!("{namespace}.{short}")
    }
}

/// A namespace split into its name and optional version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Namespace {
    /// The namespace without any version suffix, e.g. `org.example`.
    pub name: String,
    /// The version, e.g. `1.0.0`, when the namespace was versioned.
    pub version: Option<String>,
}

/// Parses a possibly-versioned namespace such as `org.example@1.0.0`.
///
/// Returns [`ConcertoError::IllegalModel`] if more than one `@` is present.
///
/// ```
/// # use concerto_core::model_util::parse_namespace;
/// let ns = parse_namespace("org.example@1.0.0").unwrap();
/// assert_eq!(ns.name, "org.example");
/// assert_eq!(ns.version.as_deref(), Some("1.0.0"));
/// ```
pub fn parse_namespace(namespace: &str) -> Result<Namespace> {
    let mut parts = namespace.splitn(3, '@');
    let name = parts.next().unwrap_or("").to_string();
    match (parts.next(), parts.next()) {
        (None, _) => Ok(Namespace {
            name,
            version: None,
        }),
        (Some(version), None) => Ok(Namespace {
            name,
            version: Some(version.to_string()),
        }),
        (Some(_), Some(_)) => Err(ConcertoError::IllegalModel {
            message: format!("invalid namespace (multiple '@'): {namespace}"),
            file_name: None,
            location: None,
        }),
    }
}

/// Returns `true` for the six Concerto primitive type names.
///
/// ```
/// # use concerto_core::model_util::is_primitive_type;
/// assert!(is_primitive_type("String"));
/// assert!(!is_primitive_type("Person"));
/// ```
pub fn is_primitive_type(type_name: &str) -> bool {
    PRIMITIVE_TYPES.contains(&type_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_name_splits_on_last_dot() {
        assert_eq!(short_name("org.example@1.0.0.Person"), "Person");
        assert_eq!(short_name("a.b.c.D"), "D");
        assert_eq!(short_name("Person"), "Person");
    }

    #[test]
    fn namespace_of_returns_prefix() {
        assert_eq!(
            namespace_of("org.example@1.0.0.Person"),
            "org.example@1.0.0"
        );
        assert_eq!(namespace_of("Person"), "");
    }

    #[test]
    fn qualify_round_trips() {
        let fqn = qualify("org.example@1.0.0", "Person");
        assert_eq!(fqn, "org.example@1.0.0.Person");
        assert_eq!(namespace_of(&fqn), "org.example@1.0.0");
        assert_eq!(short_name(&fqn), "Person");
        assert_eq!(qualify("", "String"), "String");
    }

    #[test]
    fn parse_namespace_handles_version() {
        let ns = parse_namespace("org.example@1.0.0").unwrap();
        assert_eq!(ns.name, "org.example");
        assert_eq!(ns.version.as_deref(), Some("1.0.0"));

        let ns = parse_namespace("org.example").unwrap();
        assert_eq!(ns.name, "org.example");
        assert!(ns.version.is_none());

        assert!(parse_namespace("a@1@2").is_err());
    }

    #[test]
    fn primitive_types_are_recognised() {
        for t in ["Boolean", "String", "DateTime", "Double", "Integer", "Long"] {
            assert!(is_primitive_type(t));
        }
        assert!(!is_primitive_type("Concept"));
    }
}
