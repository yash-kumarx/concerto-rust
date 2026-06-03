//! # concerto-core
//!
//! Core of the Rust implementation of the Accord Project [Concerto] modeling
//! language. This crate owns the in-memory representation of Concerto models
//! and, in later work, the logic for validating data against them.
//!
//! It is built on top of the generated [`concerto_metamodel`] types, which it
//! wraps using the new-type pattern rather than re-deriving the schema (see
//! `AGENTS.md`).
//!
//! [Concerto]: https://concerto.accordproject.org/docs/category/specification

pub mod error;

pub use error::{ConcertoError, Result};

#[cfg(test)]
mod tests {
    use concerto_metamodel::concerto_metamodel_1_0_0 as mm;

    /// Smoke test: the generated metamodel crate is linked and a core
    /// declaration type round-trips from its JSON AST form.
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
