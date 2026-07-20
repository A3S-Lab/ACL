//! Declarative ACL document schemas and bounded validation diagnostics.

mod validator;

use std::collections::{BTreeMap, BTreeSet};

pub use validator::{validate_document, validate_document_with_limits};

/// Invalid trusted schema definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaDefinitionError {
    message: String,
}

impl SchemaDefinitionError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for SchemaDefinitionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for SchemaDefinitionError {}

/// Inclusive cardinality accepted by a schema rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cardinality {
    min: usize,
    max: Option<usize>,
}

impl Cardinality {
    /// Construct a cardinality. `None` means there is no upper bound.
    pub fn new(min: usize, max: Option<usize>) -> Result<Self, SchemaDefinitionError> {
        if max.is_some_and(|max| max < min) {
            return Err(SchemaDefinitionError::new(
                "schema cardinality maximum must be greater than or equal to its minimum",
            ));
        }
        Ok(Self { min, max })
    }

    /// Require exactly `count` values.
    pub const fn exactly(count: usize) -> Self {
        Self {
            min: count,
            max: Some(count),
        }
    }

    /// Require at least `min` values.
    pub const fn at_least(min: usize) -> Self {
        Self { min, max: None }
    }

    /// Minimum accepted value count.
    pub const fn min(self) -> usize {
        self.min
    }

    /// Maximum accepted value count, or `None` when unbounded.
    pub const fn max(self) -> Option<usize> {
        self.max
    }

    pub(crate) fn contains(self, count: usize) -> bool {
        count >= self.min && self.max.is_none_or(|max| count <= max)
    }

    pub(crate) fn describe(self, singular: &str, plural: &str) -> String {
        match self.max {
            Some(max) if max == self.min => {
                let noun = if self.min == 1 { singular } else { plural };
                format!("exactly {} {noun}", self.min)
            }
            Some(max) => format!("between {} and {max} {plural}", self.min),
            None => {
                let noun = if self.min == 1 { singular } else { plural };
                format!("at least {} {noun}", self.min)
            }
        }
    }
}

/// Schema for an ACL document or block body.
///
/// Schemas are closed by default: unknown attributes and blocks are rejected.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Schema {
    attributes: BTreeMap<String, AttributeSchema>,
    blocks: BTreeMap<String, BlockSchema>,
    allow_unknown_attributes: bool,
    allow_unknown_blocks: bool,
}

impl Schema {
    /// Construct an empty, closed schema.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add or replace one attribute rule.
    pub fn attribute(mut self, name: impl Into<String>, attribute: AttributeSchema) -> Self {
        self.attributes.insert(name.into(), attribute);
        self
    }

    /// Add or replace one nested block rule.
    pub fn block(mut self, name: impl Into<String>, block: BlockSchema) -> Self {
        self.blocks.insert(name.into(), block);
        self
    }

    /// Configure whether undeclared attributes are accepted.
    pub fn allow_unknown_attributes(mut self, allow: bool) -> Self {
        self.allow_unknown_attributes = allow;
        self
    }

    /// Configure whether undeclared blocks are accepted.
    pub fn allow_unknown_blocks(mut self, allow: bool) -> Self {
        self.allow_unknown_blocks = allow;
        self
    }
}

/// Required or optional attribute value rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeSchema {
    required: bool,
    value: ValueSchema,
}

impl AttributeSchema {
    /// Construct a required attribute rule.
    pub fn required(value: ValueSchema) -> Self {
        Self {
            required: true,
            value,
        }
    }

    /// Construct an optional attribute rule.
    pub fn optional(value: ValueSchema) -> Self {
        Self {
            required: false,
            value,
        }
    }
}

/// Nested block body, occurrence, and label rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockSchema {
    body: Schema,
    occurrences: Cardinality,
    labels: Cardinality,
}

impl BlockSchema {
    /// Construct an optional repeatable block with no labels.
    pub fn new(body: Schema) -> Self {
        Self {
            body,
            occurrences: Cardinality::at_least(0),
            labels: Cardinality::exactly(0),
        }
    }

    /// Configure how many blocks with this name may occur.
    pub fn occurrences(mut self, occurrences: Cardinality) -> Self {
        self.occurrences = occurrences;
        self
    }

    /// Configure how many labels each matching block must have.
    pub fn labels(mut self, labels: Cardinality) -> Self {
        self.labels = labels;
        self
    }
}

/// Schema for one object value.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectSchema {
    fields: BTreeMap<String, AttributeSchema>,
    allow_unknown_fields: bool,
}

impl ObjectSchema {
    /// Construct an empty, closed object schema.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add or replace one object field rule.
    pub fn field(mut self, name: impl Into<String>, field: AttributeSchema) -> Self {
        self.fields.insert(name.into(), field);
        self
    }

    /// Configure whether undeclared object fields are accepted.
    pub fn allow_unknown_fields(mut self, allow: bool) -> Self {
        self.allow_unknown_fields = allow;
        self
    }
}

/// Schema for one ACL function call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallSchema {
    allowed_names: BTreeSet<String>,
    arguments: Cardinality,
    argument: Box<ValueSchema>,
}

impl CallSchema {
    /// Construct an unrestricted function-name rule with any number of arguments.
    pub fn new() -> Self {
        Self {
            allowed_names: BTreeSet::new(),
            arguments: Cardinality::at_least(0),
            argument: Box::new(ValueSchema::any()),
        }
    }

    /// Permit one function name. No names means every function name is accepted.
    pub fn allowed_name(mut self, name: impl Into<String>) -> Self {
        self.allowed_names.insert(name.into());
        self
    }

    /// Configure the argument cardinality and the schema applied to every argument.
    pub fn arguments(mut self, arguments: Cardinality, argument: ValueSchema) -> Self {
        self.arguments = arguments;
        self.argument = Box::new(argument);
        self
    }
}

impl Default for CallSchema {
    fn default() -> Self {
        Self::new()
    }
}

/// Recursive ACL value rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueSchema {
    kind: ValueSchemaKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ValueSchemaKind {
    Any,
    String,
    Number,
    Bool,
    Null,
    List(Box<ValueSchema>),
    Object(ObjectSchema),
    Call(CallSchema),
    OneOf(Vec<ValueSchema>),
}

impl ValueSchema {
    /// Accept any ACL value.
    pub fn any() -> Self {
        Self {
            kind: ValueSchemaKind::Any,
        }
    }

    /// Accept a string.
    pub fn string() -> Self {
        Self {
            kind: ValueSchemaKind::String,
        }
    }

    /// Accept a number.
    pub fn number() -> Self {
        Self {
            kind: ValueSchemaKind::Number,
        }
    }

    /// Accept a boolean.
    pub fn bool() -> Self {
        Self {
            kind: ValueSchemaKind::Bool,
        }
    }

    /// Accept null.
    pub fn null() -> Self {
        Self {
            kind: ValueSchemaKind::Null,
        }
    }

    /// Accept a list whose items match `item`.
    pub fn list(item: ValueSchema) -> Self {
        Self {
            kind: ValueSchemaKind::List(Box::new(item)),
        }
    }

    /// Accept an object matching `object`.
    pub fn object(object: ObjectSchema) -> Self {
        Self {
            kind: ValueSchemaKind::Object(object),
        }
    }

    /// Accept a function call matching `call`.
    pub fn call(call: CallSchema) -> Self {
        Self {
            kind: ValueSchemaKind::Call(call),
        }
    }

    /// Accept a value matching at least one variant.
    pub fn one_of(variants: Vec<ValueSchema>) -> Result<Self, SchemaDefinitionError> {
        if variants.is_empty() {
            return Err(SchemaDefinitionError::new(
                "schema union must contain at least one variant",
            ));
        }
        Ok(Self {
            kind: ValueSchemaKind::OneOf(variants),
        })
    }

    fn kind(&self) -> &ValueSchemaKind {
        &self.kind
    }
}

/// Stable machine-readable category for one schema diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaDiagnosticCode {
    UnknownAttribute,
    DuplicateAttribute,
    MissingAttribute,
    UnknownBlock,
    BlockCount,
    LabelCount,
    ValueType,
    UnknownObjectField,
    DuplicateObjectField,
    MissingObjectField,
    CallName,
    CallArgumentCount,
}

impl SchemaDiagnosticCode {
    /// Return the cross-SDK wire representation of this diagnostic code.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UnknownAttribute => "acl.schema.unknown_attribute",
            Self::DuplicateAttribute => "acl.schema.duplicate_attribute",
            Self::MissingAttribute => "acl.schema.missing_attribute",
            Self::UnknownBlock => "acl.schema.unknown_block",
            Self::BlockCount => "acl.schema.block_count",
            Self::LabelCount => "acl.schema.label_count",
            Self::ValueType => "acl.schema.value_type",
            Self::UnknownObjectField => "acl.schema.unknown_object_field",
            Self::DuplicateObjectField => "acl.schema.duplicate_object_field",
            Self::MissingObjectField => "acl.schema.missing_object_field",
            Self::CallName => "acl.schema.call_name",
            Self::CallArgumentCount => "acl.schema.call_argument_count",
        }
    }
}

impl std::fmt::Display for SchemaDiagnosticCode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Structured schema diagnostic with a stable logical document path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaDiagnostic {
    pub code: SchemaDiagnosticCode,
    pub message: String,
    pub path: String,
}

/// Bounded collection of schema diagnostics.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SchemaReport {
    pub diagnostics: Vec<SchemaDiagnostic>,
    pub truncated: bool,
}

impl SchemaReport {
    /// Return true when no schema diagnostic was observed.
    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty() && !self.truncated
    }
}
