//! The in-memory representation of a Concerto model.
//!
//! Types here wrap the generated [`concerto_metamodel`] structs and expose
//! them as a small set of sum types ([`Declaration`], [`Property`]) with
//! inherent accessors, built directly from a model's JSON AST.

pub mod declaration;
pub mod property;

pub use declaration::{ClassDeclaration, ClassKind, Declaration, ScalarDeclaration};
pub use property::Property;

/// Reads the `$class` discriminator from an AST node, returning `""` when it
/// is absent. The model layer keys every sum-type dispatch off this value.
pub(crate) fn declared_class(value: &serde_json::Value) -> &str {
    value.get("$class").and_then(|v| v.as_str()).unwrap_or("")
}
