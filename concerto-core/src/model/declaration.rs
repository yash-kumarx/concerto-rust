//! Typed representation of the declarations in a model file.
//!
//! Concerto's JavaScript runtime models declarations as a class hierarchy
//! (`Concept`/`Asset`/`Participant`/`Transaction`/`Event` all extend a common
//! class declaration). The five class-like declarations are structurally
//! identical, so rather than five near-duplicate structs we use a single
//! [`ClassDeclaration`] carrying a [`ClassKind`] tag. Scalars and maps, which
//! have different shapes, are their own variants of the [`Declaration`] sum
//! type. This keeps the representation a set of enums with inherent methods
//! (`AGENTS.md`: prefer sum types over complex trait hierarchies).

use concerto_metamodel::concerto_metamodel_1_0_0 as mm;

use crate::error::{ConcertoError, Result};
use crate::model::declared_class;
use crate::model::property::Property;
use crate::model_util::short_name;

/// Which class-like declaration a [`ClassDeclaration`] represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassKind {
    /// A `concept`.
    Concept,
    /// An `asset` (system-identifiable).
    Asset,
    /// A `participant` (system-identifiable).
    Participant,
    /// A `transaction`.
    Transaction,
    /// An `event`.
    Event,
}

impl ClassKind {
    /// The metamodel `$class` short name for this kind.
    pub fn declaration_kind(self) -> &'static str {
        match self {
            Self::Concept => "ConceptDeclaration",
            Self::Asset => "AssetDeclaration",
            Self::Participant => "ParticipantDeclaration",
            Self::Transaction => "TransactionDeclaration",
            Self::Event => "EventDeclaration",
        }
    }

    fn from_short(short: &str) -> Option<Self> {
        Some(match short {
            "ConceptDeclaration" => Self::Concept,
            "AssetDeclaration" => Self::Asset,
            "ParticipantDeclaration" => Self::Participant,
            "TransactionDeclaration" => Self::Transaction,
            "EventDeclaration" => Self::Event,
            _ => return None,
        })
    }
}

/// Header fields shared by every class-like declaration. Properties are parsed
/// separately into the typed [`Property`] sum type, so this intentionally does
/// not deserialize them.
#[derive(serde::Deserialize)]
struct ClassHeader {
    name: String,
    #[serde(rename = "isAbstract", default)]
    is_abstract: bool,
    #[serde(rename = "superType")]
    super_type: Option<mm::TypeIdentifier>,
    identified: Option<mm::Identified>,
    decorators: Option<Vec<mm::Decorator>>,
    location: Option<mm::Range>,
}

/// A concept-like declaration: concept, asset, participant, transaction or
/// event, distinguished by [`ClassDeclaration::kind`].
#[derive(Debug, Clone)]
pub struct ClassDeclaration {
    kind: ClassKind,
    name: String,
    is_abstract: bool,
    super_type: Option<mm::TypeIdentifier>,
    identified: Option<mm::Identified>,
    properties: Vec<Property>,
    decorators: Vec<mm::Decorator>,
    location: Option<mm::Range>,
}

impl ClassDeclaration {
    /// The kind of class-like declaration this is.
    pub fn kind(&self) -> ClassKind {
        self.kind
    }

    /// The declaration's short name (without namespace).
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Whether the declaration is abstract (cannot be instantiated directly).
    pub fn is_abstract(&self) -> bool {
        self.is_abstract
    }

    /// The directly-declared super type, if the declaration extends another.
    pub fn super_type(&self) -> Option<&mm::TypeIdentifier> {
        self.super_type.as_ref()
    }

    /// The properties declared directly on this type (not inherited ones).
    pub fn own_properties(&self) -> &[Property] {
        &self.properties
    }

    /// The decorators attached to this declaration.
    pub fn decorators(&self) -> &[mm::Decorator] {
        &self.decorators
    }

    /// The source location, if the AST carried one.
    pub fn location(&self) -> Option<&mm::Range> {
        self.location.as_ref()
    }

    /// Whether the declaration carries an identity (system or explicit).
    pub fn is_identified(&self) -> bool {
        self.identified.is_some()
    }

    fn parse(kind: ClassKind, value: &serde_json::Value) -> Result<Self> {
        let header: ClassHeader =
            serde_json::from_value(value.clone()).map_err(|e| ConcertoError::IllegalModel {
                message: format!("invalid {}: {e}", kind.declaration_kind()),
                file_name: None,
                location: None,
            })?;
        Ok(Self {
            kind,
            name: header.name,
            is_abstract: header.is_abstract,
            super_type: header.super_type,
            identified: header.identified,
            properties: parse_properties(value)?,
            decorators: header.decorators.unwrap_or_default(),
            location: header.location,
        })
    }
}

/// A scalar declaration: a named alias over a primitive, possibly with a
/// validator. The variant selects which primitive is aliased.
#[derive(Debug, Clone)]
pub enum ScalarDeclaration {
    /// `scalar X extends Boolean`.
    Boolean(mm::BooleanScalar),
    /// `scalar X extends Integer`.
    Integer(mm::IntegerScalar),
    /// `scalar X extends Long`.
    Long(mm::LongScalar),
    /// `scalar X extends Double`.
    Double(mm::DoubleScalar),
    /// `scalar X extends String`.
    String(mm::StringScalar),
    /// `scalar X extends DateTime`.
    DateTime(mm::DateTimeScalar),
}

impl ScalarDeclaration {
    /// The scalar's short name.
    pub fn name(&self) -> &str {
        match self {
            Self::Boolean(s) => &s.name,
            Self::Integer(s) => &s.name,
            Self::Long(s) => &s.name,
            Self::Double(s) => &s.name,
            Self::String(s) => &s.name,
            Self::DateTime(s) => &s.name,
        }
    }

    /// The primitive type this scalar aliases.
    pub fn scalar_type(&self) -> &'static str {
        match self {
            Self::Boolean(_) => "Boolean",
            Self::Integer(_) => "Integer",
            Self::Long(_) => "Long",
            Self::Double(_) => "Double",
            Self::String(_) => "String",
            Self::DateTime(_) => "DateTime",
        }
    }

    fn parse(short: &str, value: &serde_json::Value) -> Result<Self> {
        let bad = |e: serde_json::Error| ConcertoError::IllegalModel {
            message: format!("invalid {short}: {e}"),
            file_name: None,
            location: None,
        };
        let v = value.clone();
        Ok(match short {
            "BooleanScalar" => Self::Boolean(serde_json::from_value(v).map_err(bad)?),
            "IntegerScalar" => Self::Integer(serde_json::from_value(v).map_err(bad)?),
            "LongScalar" => Self::Long(serde_json::from_value(v).map_err(bad)?),
            "DoubleScalar" => Self::Double(serde_json::from_value(v).map_err(bad)?),
            "StringScalar" => Self::String(serde_json::from_value(v).map_err(bad)?),
            "DateTimeScalar" => Self::DateTime(serde_json::from_value(v).map_err(bad)?),
            other => {
                return Err(ConcertoError::IllegalModel {
                    message: format!("unknown scalar type: {other}"),
                    file_name: None,
                    location: None,
                });
            }
        })
    }
}

/// A top-level declaration within a model file.
#[derive(Debug, Clone)]
pub enum Declaration {
    /// A concept-like declaration (see [`ClassDeclaration`]).
    Class(ClassDeclaration),
    /// An enumeration.
    Enum(mm::EnumDeclaration),
    /// A scalar alias over a primitive.
    Scalar(ScalarDeclaration),
    /// A map type.
    Map(mm::MapDeclaration),
}

impl Declaration {
    /// The declaration's short name.
    pub fn name(&self) -> &str {
        match self {
            Self::Class(c) => c.name(),
            Self::Enum(e) => &e.name,
            Self::Scalar(s) => s.name(),
            Self::Map(m) => &m.name,
        }
    }

    /// The metamodel `$class` short name for this declaration.
    pub fn declaration_kind(&self) -> &'static str {
        match self {
            Self::Class(c) => c.kind().declaration_kind(),
            Self::Enum(_) => "EnumDeclaration",
            Self::Scalar(_) => "ScalarDeclaration",
            Self::Map(_) => "MapDeclaration",
        }
    }

    /// Borrow this as a [`ClassDeclaration`], if it is one.
    pub fn as_class(&self) -> Option<&ClassDeclaration> {
        match self {
            Self::Class(c) => Some(c),
            _ => None,
        }
    }

    /// Borrow this as a [`ScalarDeclaration`], if it is one.
    pub fn as_scalar(&self) -> Option<&ScalarDeclaration> {
        match self {
            Self::Scalar(s) => Some(s),
            _ => None,
        }
    }

    /// `true` if this is an enum declaration.
    pub fn is_enum(&self) -> bool {
        matches!(self, Self::Enum(_))
    }

    /// `true` if this is a map declaration.
    pub fn is_map(&self) -> bool {
        matches!(self, Self::Map(_))
    }
}

fn parse_properties(value: &serde_json::Value) -> Result<Vec<Property>> {
    match value.get("properties") {
        Some(serde_json::Value::Array(arr)) => arr.iter().map(Property::try_from).collect(),
        _ => Ok(Vec::new()),
    }
}

impl TryFrom<&serde_json::Value> for Declaration {
    type Error = ConcertoError;

    fn try_from(value: &serde_json::Value) -> Result<Self> {
        let class = declared_class(value);
        let kind = short_name(class);

        if let Some(class_kind) = ClassKind::from_short(kind) {
            return Ok(Self::Class(ClassDeclaration::parse(class_kind, value)?));
        }

        Ok(match kind {
            "EnumDeclaration" => {
                Self::Enum(serde_json::from_value(value.clone()).map_err(|e| {
                    ConcertoError::IllegalModel {
                        message: format!("invalid EnumDeclaration: {e}"),
                        file_name: None,
                        location: None,
                    }
                })?)
            }
            "MapDeclaration" => Self::Map(serde_json::from_value(value.clone()).map_err(|e| {
                ConcertoError::IllegalModel {
                    message: format!("invalid MapDeclaration: {e}"),
                    file_name: None,
                    location: None,
                }
            })?),
            s if s.ends_with("Scalar") => Self::Scalar(ScalarDeclaration::parse(s, value)?),
            other => {
                return Err(ConcertoError::IllegalModel {
                    message: format!("unknown declaration type: {other}"),
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

    fn decl(json: serde_json::Value) -> Declaration {
        Declaration::try_from(&json).expect("valid declaration")
    }

    #[test]
    fn parses_concept_with_typed_properties() {
        let d = decl(serde_json::json!({
            "$class": "concerto.metamodel@1.0.0.ConceptDeclaration",
            "name": "Person",
            "isAbstract": false,
            "superType": { "$class": "concerto.metamodel@1.0.0.TypeIdentifier", "name": "Thing" },
            "properties": [
                { "$class": "concerto.metamodel@1.0.0.StringProperty", "name": "firstName", "isArray": false, "isOptional": false },
                { "$class": "concerto.metamodel@1.0.0.IntegerProperty", "name": "age", "isArray": false, "isOptional": true }
            ]
        }));

        let c = d.as_class().expect("class");
        assert_eq!(c.kind(), ClassKind::Concept);
        assert_eq!(c.name(), "Person");
        assert!(!c.is_abstract());
        assert_eq!(c.super_type().map(|t| t.name.as_str()), Some("Thing"));
        assert_eq!(c.own_properties().len(), 2);
        assert_eq!(c.own_properties()[0].type_name(), Some("String"));
        assert!(c.own_properties()[1].is_optional());
    }

    #[test]
    fn asset_kind_is_tagged() {
        let d = decl(serde_json::json!({
            "$class": "concerto.metamodel@1.0.0.AssetDeclaration",
            "name": "Vehicle",
            "isAbstract": false,
            "properties": []
        }));
        assert_eq!(d.declaration_kind(), "AssetDeclaration");
        assert_eq!(d.as_class().unwrap().kind(), ClassKind::Asset);
    }

    #[test]
    fn parses_enum_and_scalar_and_map() {
        let e = decl(serde_json::json!({
            "$class": "concerto.metamodel@1.0.0.EnumDeclaration",
            "name": "Color",
            "properties": [
                { "$class": "concerto.metamodel@1.0.0.EnumProperty", "name": "RED" },
                { "$class": "concerto.metamodel@1.0.0.EnumProperty", "name": "GREEN" }
            ]
        }));
        assert!(e.is_enum());
        assert_eq!(e.name(), "Color");

        let s = decl(serde_json::json!({
            "$class": "concerto.metamodel@1.0.0.StringScalar",
            "name": "Email",
            "validator": { "$class": "concerto.metamodel@1.0.0.StringRegexValidator", "pattern": ".*", "flags": "" }
        }));
        assert_eq!(s.as_scalar().unwrap().scalar_type(), "String");
        assert_eq!(s.name(), "Email");

        let m = decl(serde_json::json!({
            "$class": "concerto.metamodel@1.0.0.MapDeclaration",
            "name": "Dictionary",
            "key": { "$class": "concerto.metamodel@1.0.0.StringMapKeyType" },
            "value": { "$class": "concerto.metamodel@1.0.0.StringMapValueType" }
        }));
        assert!(m.is_map());
        assert_eq!(m.name(), "Dictionary");
    }

    #[test]
    fn unknown_declaration_kind_errors() {
        assert!(
            Declaration::try_from(&serde_json::json!({
                "$class": "concerto.metamodel@1.0.0.WidgetDeclaration",
                "name": "X"
            }))
            .is_err()
        );
    }
}
