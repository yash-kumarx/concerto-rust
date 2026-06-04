//! A single parsed Concerto model file.
//!
//! A [`ModelFile`] owns the declarations of one namespace plus its imports,
//! and indexes its declarations by short name. It can resolve a short type
//! name to a fully-qualified name using its **local** declarations and
//! **explicit** imports; wildcard (`ns.*`) imports are left to the
//! [`ModelManager`](crate::model_manager::ModelManager), which can see the
//! imported namespaces.

use std::collections::HashMap;

use crate::error::{ConcertoError, Result};
use crate::model::declaration::Declaration;
use crate::model::import::Import;
use crate::model_util::{is_primitive_type, parse_namespace, qualify};

/// A parsed model file for one namespace.
#[derive(Debug, Clone)]
pub struct ModelFile {
    namespace: String,
    version: Option<String>,
    imports: Vec<Import>,
    declarations: Vec<Declaration>,
    local_types: HashMap<String, usize>,
    file_name: Option<String>,
}

impl ModelFile {
    /// Parses a model file from the JSON AST of a `concerto.metamodel@….Model`.
    pub fn from_json(value: &serde_json::Value, file_name: Option<String>) -> Result<Self> {
        let namespace = value
            .get("namespace")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ConcertoError::IllegalModel {
                message: "model missing 'namespace'".into(),
                file_name: file_name.clone(),
                location: None,
            })?
            .to_string();

        let version = parse_namespace(&namespace)?.version;

        let imports = match value.get("imports") {
            Some(serde_json::Value::Array(arr)) => arr
                .iter()
                .map(Import::try_from)
                .collect::<Result<Vec<_>>>()?,
            _ => Vec::new(),
        };

        let mut declarations = Vec::new();
        let mut local_types = HashMap::new();
        if let Some(serde_json::Value::Array(arr)) = value.get("declarations") {
            for raw in arr {
                let decl = Declaration::try_from(raw).map_err(|e| annotate(e, &file_name))?;
                if local_types
                    .insert(decl.name().to_string(), declarations.len())
                    .is_some()
                {
                    return Err(ConcertoError::IllegalModel {
                        message: format!("duplicate declaration '{}' in {namespace}", decl.name()),
                        file_name: file_name.clone(),
                        location: None,
                    });
                }
                declarations.push(decl);
            }
        }

        Ok(Self {
            namespace,
            version,
            imports,
            declarations,
            local_types,
            file_name,
        })
    }

    /// The full (possibly versioned) namespace, e.g. `org.example@1.0.0`.
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// The namespace version, if any.
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    /// The originating file name, if one was supplied.
    pub fn file_name(&self) -> Option<&str> {
        self.file_name.as_deref()
    }

    /// All declarations in this file, in source order.
    pub fn declarations(&self) -> &[Declaration] {
        &self.declarations
    }

    /// This file's import statements.
    pub fn imports(&self) -> &[Import] {
        &self.imports
    }

    /// Looks up a declaration by its short (local) name.
    pub fn local_declaration(&self, short: &str) -> Option<&Declaration> {
        self.local_types.get(short).map(|&i| &self.declarations[i])
    }

    /// `true` if this is the built-in `concerto` system namespace.
    pub fn is_system(&self) -> bool {
        self.namespace == "concerto" || self.namespace.starts_with("concerto@")
    }

    /// Resolves a short type name to a fully-qualified name using primitives,
    /// local declarations, and explicit imports. Returns `None` when only a
    /// wildcard import could match (the model manager resolves those).
    pub fn resolve_local(&self, short: &str) -> Option<String> {
        if is_primitive_type(short) {
            return Some(short.to_string());
        }
        if self.local_types.contains_key(short) {
            return Some(qualify(&self.namespace, short));
        }
        self.imports.iter().find_map(|imp| imp.resolve(short))
    }
}

/// Attaches this file's name to an `IllegalModel` error raised while parsing
/// one of its declarations.
fn annotate(err: ConcertoError, file_name: &Option<String>) -> ConcertoError {
    match err {
        ConcertoError::IllegalModel {
            message, location, ..
        } => ConcertoError::IllegalModel {
            message,
            file_name: file_name.clone(),
            location,
        },
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ModelFile {
        ModelFile::from_json(
            &serde_json::json!({
                "$class": "concerto.metamodel@1.0.0.Model",
                "namespace": "org.example@1.0.0",
                "imports": [
                    { "$class": "concerto.metamodel@1.0.0.ImportType",
                      "namespace": "org.common@1.0.0", "name": "Address" }
                ],
                "declarations": [
                    { "$class": "concerto.metamodel@1.0.0.ConceptDeclaration",
                      "name": "Person", "isAbstract": false, "properties": [] }
                ]
            }),
            Some("example.cto".into()),
        )
        .unwrap()
    }

    #[test]
    fn parses_namespace_imports_and_declarations() {
        let mf = sample();
        assert_eq!(mf.namespace(), "org.example@1.0.0");
        assert_eq!(mf.version(), Some("1.0.0"));
        assert_eq!(mf.declarations().len(), 1);
        assert_eq!(mf.imports().len(), 1);
        assert!(mf.local_declaration("Person").is_some());
        assert!(!mf.is_system());
    }

    #[test]
    fn resolves_local_primitive_and_import() {
        let mf = sample();
        assert_eq!(
            mf.resolve_local("Person").as_deref(),
            Some("org.example@1.0.0.Person")
        );
        assert_eq!(mf.resolve_local("String").as_deref(), Some("String"));
        assert_eq!(
            mf.resolve_local("Address").as_deref(),
            Some("org.common@1.0.0.Address")
        );
        assert_eq!(mf.resolve_local("Missing"), None);
    }

    #[test]
    fn duplicate_declaration_is_rejected() {
        let err = ModelFile::from_json(
            &serde_json::json!({
                "$class": "concerto.metamodel@1.0.0.Model",
                "namespace": "org.dup@1.0.0",
                "declarations": [
                    { "$class": "concerto.metamodel@1.0.0.ConceptDeclaration", "name": "A", "isAbstract": false, "properties": [] },
                    { "$class": "concerto.metamodel@1.0.0.ConceptDeclaration", "name": "A", "isAbstract": false, "properties": [] }
                ]
            }),
            None,
        );
        assert!(err.is_err());
    }

    #[test]
    fn missing_namespace_is_rejected() {
        let err = ModelFile::from_json(
            &serde_json::json!({ "$class": "concerto.metamodel@1.0.0.Model" }),
            None,
        );
        assert!(err.is_err());
    }
}
