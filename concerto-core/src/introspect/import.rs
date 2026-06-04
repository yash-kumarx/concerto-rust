//! A model's imports, typed.
//!
//! Same story as properties: deserializing through the metamodel flattens every
//! import into one base `Import` and throws away the type names it brought in.
//! So we read each import again from its JSON into this [`Import`] enum,
//! switching on the `$class`. That's what lets the introspect layer turn a
//! short name back into the namespace it came from.

use crate::error::{ConcertoError, Result};
use crate::introspect::declared_class;
use crate::model_util::{qualify, short_name};

/// A single import statement in a model file.
#[derive(Debug, Clone)]
pub enum Import {
    /// `import ns.*`, the whole namespace.
    All {
        /// The namespace being imported.
        namespace: String,
    },
    /// `import ns.Name`, a single type.
    Type {
        /// Where the type comes from.
        namespace: String,
        /// Its short name.
        name: String,
    },
    /// `import ns.{A, B}`, a few named types, some maybe aliased.
    Types {
        /// The namespace the types are imported from.
        namespace: String,
        /// The imported short type names.
        names: Vec<String>,
        /// `(local_alias, original_name)` pairs for aliased imports.
        aliases: Vec<(String, String)>,
    },
}

impl Import {
    /// The namespace this import refers to.
    pub fn namespace(&self) -> &str {
        match self {
            Self::All { namespace }
            | Self::Type { namespace, .. }
            | Self::Types { namespace, .. } => namespace,
        }
    }

    /// True for a `ns.*` import. We can't resolve those on our own, we'd need
    /// to look at what the other namespace actually declares.
    pub fn is_wildcard(&self) -> bool {
        matches!(self, Self::All { .. })
    }

    /// Turns a short name into its full name, but only if this import mentions
    /// it by name. Wildcards always give back `None` here, since resolving them
    /// means going and reading the other model file.
    pub fn resolve(&self, short: &str) -> Option<String> {
        match self {
            Self::All { .. } => None,
            Self::Type { namespace, name } if name == short => Some(qualify(namespace, name)),
            Self::Type { .. } => None,
            Self::Types {
                namespace,
                names,
                aliases,
            } => {
                if let Some((_, original)) = aliases.iter().find(|(alias, _)| alias == short) {
                    return Some(qualify(namespace, original));
                }
                if names.iter().any(|n| n == short) {
                    return Some(qualify(namespace, short));
                }
                None
            }
        }
    }
}

impl TryFrom<&serde_json::Value> for Import {
    type Error = ConcertoError;

    fn try_from(value: &serde_json::Value) -> Result<Self> {
        let kind = short_name(declared_class(value));

        let namespace = value
            .get("namespace")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ConcertoError::IllegalModel {
                message: format!("import ({kind}) missing 'namespace'"),
                file_name: None,
                location: None,
            })?
            .to_string();

        Ok(match kind {
            // Bare `Import` is the abstract base; treat it like `ImportAll`.
            "ImportAll" | "Import" => Self::All { namespace },
            "ImportType" => {
                let name = value
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ConcertoError::IllegalModel {
                        message: "ImportType missing 'name'".into(),
                        file_name: None,
                        location: None,
                    })?
                    .to_string();
                Self::Type { namespace, name }
            }
            "ImportTypes" => {
                let names = value
                    .get("types")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default();
                let aliases = value
                    .get("aliasedTypes")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|a| {
                                let alias = a.get("aliasedName").and_then(|v| v.as_str())?;
                                let original = a.get("name").and_then(|v| v.as_str())?;
                                Some((alias.to_string(), original.to_string()))
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                Self::Types {
                    namespace,
                    names,
                    aliases,
                }
            }
            other => {
                return Err(ConcertoError::IllegalModel {
                    message: format!("unknown import type: {other}"),
                    file_name: None,
                    location: None,
                });
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_named_import() {
        let imp = Import::try_from(&serde_json::json!({
            "$class": "concerto.metamodel@1.0.0.ImportType",
            "namespace": "org.acme@1.0.0",
            "name": "Person"
        }))
        .unwrap();
        assert_eq!(imp.namespace(), "org.acme@1.0.0");
        assert_eq!(
            imp.resolve("Person").as_deref(),
            Some("org.acme@1.0.0.Person")
        );
        assert_eq!(imp.resolve("Other"), None);
    }

    #[test]
    fn resolves_multi_import_with_alias() {
        let imp = Import::try_from(&serde_json::json!({
            "$class": "concerto.metamodel@1.0.0.ImportTypes",
            "namespace": "org.acme@1.0.0",
            "types": ["A", "B"],
            "aliasedTypes": [
                { "$class": "concerto.metamodel@1.0.0.AliasedType", "name": "B", "aliasedName": "Bee" }
            ]
        }))
        .unwrap();
        assert_eq!(imp.resolve("A").as_deref(), Some("org.acme@1.0.0.A"));
        assert_eq!(imp.resolve("Bee").as_deref(), Some("org.acme@1.0.0.B"));
        assert_eq!(imp.resolve("C"), None);
    }

    #[test]
    fn wildcard_import_defers_resolution() {
        let imp = Import::try_from(&serde_json::json!({
            "$class": "concerto.metamodel@1.0.0.ImportAll",
            "namespace": "org.acme@1.0.0"
        }))
        .unwrap();
        assert!(imp.is_wildcard());
        assert_eq!(imp.resolve("Anything"), None);
        assert_eq!(imp.namespace(), "org.acme@1.0.0");
    }
}
