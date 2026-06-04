//! # concerto-core
//!
//! The heart of the Rust Concerto implementation. This crate holds the
//! in-memory picture of a Concerto schema and the type lookups built on top of
//! it. Data validation will live here too, but that part isn't written yet.
//!
//! Everything sits on top of the generated [`concerto_metamodel`] types. We
//! wrap those in our own enums rather than redefining the schema by hand.
//!
//! [Concerto]: https://concerto.accordproject.org/docs/category/specification

pub mod error;
pub mod introspect;
pub mod model_manager;
pub mod model_util;
pub mod rootmodel;

pub use error::{ConcertoError, Result};
pub use introspect::{
    ClassDeclaration, ClassKind, Declaration, Import, ModelFile, Property, ScalarDeclaration,
};
pub use model_manager::ModelManager;

#[cfg(test)]
mod tests {
    use concerto_metamodel::concerto_metamodel_1_0_0 as mm;

    // Quick check that the metamodel crate is actually wired in and a
    // declaration survives a round-trip from its JSON form.
    #[test]
    fn metamodel_crate_is_linked_and_deserializes() {
        let json = serde_json::json!({
            "$class": "concerto.metamodel@1.0.0.ConceptDeclaration",
            "name": "Person",
            "isAbstract": false,
            "properties": []
        });

        let decl: mm::ConceptDeclaration =
            serde_json::from_value(json).expect("valid ConceptDeclaration AST");

        assert_eq!(decl.name, "Person");
        assert!(!decl.is_abstract);
    }
}
