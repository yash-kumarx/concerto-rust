//! Midpoint demo: load a Concerto model and introspect it.
//!
//! Run with:
//!     cargo run -p concerto-core --example introspect

use concerto_core::{ModelManager, Property, Result};
use serde_json::json;

fn main() -> Result<()> {
    // A model manager always starts with the built-in `concerto@1.0.0` system
    // model loaded (the five base types every user type extends).
    let mut models = ModelManager::new()?;

    // Load a small model: Person <- Employee <- Manager, plus an enum.
    models.add_model(
        &json!({
            "$class": "concerto.metamodel@1.0.0.Model",
            "namespace": "org.acme@1.0.0",
            "imports": [],
            "declarations": [
                {
                    "$class": "concerto.metamodel@1.0.0.ConceptDeclaration",
                    "name": "Person", "isAbstract": false,
                    "properties": [
                        { "$class": "concerto.metamodel@1.0.0.StringProperty",  "name": "firstName", "isArray": false, "isOptional": false },
                        { "$class": "concerto.metamodel@1.0.0.IntegerProperty", "name": "age",       "isArray": false, "isOptional": true }
                    ]
                },
                {
                    "$class": "concerto.metamodel@1.0.0.ConceptDeclaration",
                    "name": "Employee", "isAbstract": false,
                    "superType": { "$class": "concerto.metamodel@1.0.0.TypeIdentifier", "name": "Person" },
                    "properties": [
                        { "$class": "concerto.metamodel@1.0.0.DoubleProperty", "name": "salary", "isArray": false, "isOptional": false }
                    ]
                },
                {
                    "$class": "concerto.metamodel@1.0.0.ConceptDeclaration",
                    "name": "Manager", "isAbstract": false,
                    "superType": { "$class": "concerto.metamodel@1.0.0.TypeIdentifier", "name": "Employee" },
                    "properties": [
                        { "$class": "concerto.metamodel@1.0.0.StringProperty", "name": "title",   "isArray": false, "isOptional": true },
                        { "$class": "concerto.metamodel@1.0.0.ObjectProperty", "name": "reports", "isArray": true,  "isOptional": false,
                          "type": { "$class": "concerto.metamodel@1.0.0.TypeIdentifier", "name": "Person" } }
                    ]
                },
                {
                    "$class": "concerto.metamodel@1.0.0.EnumDeclaration",
                    "name": "Department",
                    "properties": [
                        { "$class": "concerto.metamodel@1.0.0.EnumProperty", "name": "ENGINEERING" },
                        { "$class": "concerto.metamodel@1.0.0.EnumProperty", "name": "SALES" }
                    ]
                }
            ]
        }),
        Some("acme.cto".into()),
    )?;

    // 1) What did we load?
    let mf = models
        .model_file("org.acme@1.0.0")
        .expect("model is loaded");
    header(&format!(
        "Model {}  (version {})",
        mf.namespace(),
        mf.version().unwrap_or("-")
    ));
    for d in mf.declarations() {
        println!("  {:<12} {}", d.name(), d.declaration_kind());
    }

    // 2) Own vs. inherited properties (the inheritance walk).
    let manager = "org.acme@1.0.0.Manager";

    header("Manager: declared directly on the type");
    let class = models
        .get_declaration(manager)?
        .as_class()
        .expect("a concept");
    println!(
        "  extends {}",
        class.super_type().map(|t| t.name.as_str()).unwrap_or("-")
    );
    for p in class.own_properties() {
        println!("  {}", show(p));
    }

    header("Manager: ALL properties (walking the inheritance chain)");
    for p in models.get_all_properties(manager)? {
        println!("  {}", show(p));
    }

    // 3) Assignability: is a Manager a Person?
    header("Assignability");
    let person = "org.acme@1.0.0.Person";
    println!(
        "  Manager  ->  Person   {}",
        yesno(models.is_assignable_to(manager, person)?)
    );
    println!(
        "  Person   ->  Manager  {}",
        yesno(models.is_assignable_to(person, manager)?)
    );

    println!();
    Ok(())
}

/// Renders a property as `name: Type[]?` (array/optional markers when present).
fn show(p: &Property) -> String {
    format!(
        "{}: {}{}{}",
        p.name(),
        p.type_name().unwrap_or("?"),
        if p.is_array() { "[]" } else { "" },
        if p.is_optional() { "?" } else { "" },
    )
}

fn yesno(b: bool) -> &'static str {
    if b { "yes" } else { "no" }
}

fn header(title: &str) {
    println!("\n=== {title} ===");
}
