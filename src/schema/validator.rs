use super::{
    BlockSchema, CallSchema, ObjectSchema, Schema, SchemaDiagnostic, SchemaDiagnosticCode,
    SchemaReport, ValueSchema, ValueSchemaKind,
};
use crate::{Block, Document, ParseLimits, Value, DEFAULT_PARSE_LIMITS};
use std::collections::BTreeMap;

/// Validate a parsed ACL document with the default diagnostic budget.
pub fn validate_document(document: &Document, schema: &Schema) -> SchemaReport {
    validate_document_with_limits(document, schema, DEFAULT_PARSE_LIMITS)
}

/// Validate a parsed ACL document with an explicit diagnostic budget.
///
/// Only [`ParseLimits::max_diagnostics`] applies because the document has
/// already passed parser admission.
pub fn validate_document_with_limits(
    document: &Document,
    schema: &Schema,
    limits: ParseLimits,
) -> SchemaReport {
    let mut validator = Validator {
        report: SchemaReport::default(),
        max_diagnostics: limits.max_diagnostics,
    };
    validator.validate_document(document, schema);
    validator.report
}

struct Validator {
    report: SchemaReport,
    max_diagnostics: usize,
}

impl Validator {
    fn validate_document(&mut self, document: &Document, schema: &Schema) -> bool {
        let mut attributes: BTreeMap<String, Vec<&Value>> = BTreeMap::new();
        let mut blocks = Vec::new();
        for block in &document.blocks {
            if let Some((name, value)) = bare_attribute(block) {
                attributes.entry(name.to_string()).or_default().push(value);
            } else {
                blocks.push(block);
            }
        }
        self.validate_body(&attributes, &blocks, schema, "$")
    }

    fn validate_body(
        &mut self,
        attributes: &BTreeMap<String, Vec<&Value>>,
        blocks: &[&Block],
        schema: &Schema,
        path: &str,
    ) -> bool {
        for (name, attribute_schema) in &schema.attributes {
            if attribute_schema.required
                && !attributes.contains_key(name)
                && !self.record(
                    SchemaDiagnosticCode::MissingAttribute,
                    "Required attribute is missing",
                    format!("{path}.attributes.{name}"),
                )
            {
                return false;
            }
        }

        for (name, values) in attributes {
            let attribute_path = format!("{path}.attributes.{name}");
            if values.len() > 1
                && !self.record(
                    SchemaDiagnosticCode::DuplicateAttribute,
                    "Attribute appears more than once",
                    attribute_path.clone(),
                )
            {
                return false;
            }
            if let Some(attribute_schema) = schema.attributes.get(name) {
                let Some(value) = values.last() else {
                    continue;
                };
                if !self.validate_value(value, &attribute_schema.value, &attribute_path) {
                    return false;
                }
            } else if !schema.allow_unknown_attributes
                && !self.record(
                    SchemaDiagnosticCode::UnknownAttribute,
                    "Attribute is not allowed by the schema",
                    attribute_path,
                )
            {
                return false;
            }
        }

        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        for block in blocks {
            *counts.entry(block.name.as_str()).or_default() += 1;
        }
        for (name, block_schema) in &schema.blocks {
            let count = counts.get(name.as_str()).copied().unwrap_or_default();
            if !block_schema.occurrences.contains(count)
                && !self.record(
                    SchemaDiagnosticCode::BlockCount,
                    format!(
                        "Expected {}, found {count}",
                        block_schema.occurrences.describe("block", "blocks")
                    ),
                    format!("{path}.blocks.{name}"),
                )
            {
                return false;
            }
        }

        let mut indexes: BTreeMap<&str, usize> = BTreeMap::new();
        for block in blocks {
            let index = indexes.entry(block.name.as_str()).or_default();
            let block_path = format!("{path}.blocks.{}[{}]", block.name, *index);
            *index += 1;
            if let Some(block_schema) = schema.blocks.get(&block.name) {
                if !self.validate_block(block, block_schema, &block_path) {
                    return false;
                }
            } else if !schema.allow_unknown_blocks
                && !self.record(
                    SchemaDiagnosticCode::UnknownBlock,
                    "Block is not allowed by the schema",
                    block_path,
                )
            {
                return false;
            }
        }
        true
    }

    fn validate_block(&mut self, block: &Block, schema: &BlockSchema, path: &str) -> bool {
        if !schema.labels.contains(block.labels.len())
            && !self.record(
                SchemaDiagnosticCode::LabelCount,
                format!(
                    "Expected {}, found {}",
                    schema.labels.describe("label", "labels"),
                    block.labels.len()
                ),
                format!("{path}.labels"),
            )
        {
            return false;
        }

        let attributes = block
            .attributes
            .iter()
            .map(|(name, value)| (name.clone(), vec![value]))
            .collect();
        let blocks = block.blocks.iter().collect::<Vec<_>>();
        self.validate_body(&attributes, &blocks, &schema.body, path)
    }

    fn validate_value(&mut self, value: &Value, schema: &ValueSchema, path: &str) -> bool {
        match schema.kind() {
            ValueSchemaKind::Any => true,
            ValueSchemaKind::String => {
                self.validate_primitive(value, matches!(value, Value::String(_)), "String", path)
            }
            ValueSchemaKind::Number => {
                self.validate_primitive(value, matches!(value, Value::Number(_)), "Number", path)
            }
            ValueSchemaKind::Bool => {
                self.validate_primitive(value, matches!(value, Value::Bool(_)), "Bool", path)
            }
            ValueSchemaKind::Null => {
                self.validate_primitive(value, matches!(value, Value::Null), "Null", path)
            }
            ValueSchemaKind::List(item_schema) => {
                let Value::List(items) = value else {
                    return self.type_error(value, "List", path);
                };
                for (index, item) in items.iter().enumerate() {
                    if !self.validate_value(item, item_schema, &format!("{path}.items[{index}]")) {
                        return false;
                    }
                }
                true
            }
            ValueSchemaKind::Object(object_schema) => {
                let Value::Object(fields) = value else {
                    return self.type_error(value, "Object", path);
                };
                self.validate_object(fields, object_schema, path)
            }
            ValueSchemaKind::Call(call_schema) => {
                let Value::Call(name, arguments) = value else {
                    return self.type_error(value, "Call", path);
                };
                self.validate_call(name, arguments, call_schema, path)
            }
            ValueSchemaKind::OneOf(variants) => {
                if variants.iter().any(|variant| value_matches(value, variant)) {
                    true
                } else {
                    self.record(
                        SchemaDiagnosticCode::ValueType,
                        "Value does not match any allowed schema variant",
                        path.to_string(),
                    )
                }
            }
        }
    }

    fn validate_primitive(
        &mut self,
        value: &Value,
        matches: bool,
        expected: &str,
        path: &str,
    ) -> bool {
        if matches {
            true
        } else {
            self.type_error(value, expected, path)
        }
    }

    fn type_error(&mut self, value: &Value, expected: &str, path: &str) -> bool {
        self.record(
            SchemaDiagnosticCode::ValueType,
            format!("Expected {expected}, found {}", value_kind(value)),
            path.to_string(),
        )
    }

    fn validate_object(
        &mut self,
        fields: &[(String, Value)],
        schema: &ObjectSchema,
        path: &str,
    ) -> bool {
        let mut values: BTreeMap<&str, Vec<&Value>> = BTreeMap::new();
        for (name, value) in fields {
            values.entry(name.as_str()).or_default().push(value);
        }

        for (name, field_schema) in &schema.fields {
            if field_schema.required
                && !values.contains_key(name.as_str())
                && !self.record(
                    SchemaDiagnosticCode::MissingObjectField,
                    "Required object field is missing",
                    format!("{path}.fields.{name}"),
                )
            {
                return false;
            }
        }

        for (name, field_values) in values {
            let field_path = format!("{path}.fields.{name}");
            if field_values.len() > 1
                && !self.record(
                    SchemaDiagnosticCode::DuplicateObjectField,
                    "Object field appears more than once",
                    field_path.clone(),
                )
            {
                return false;
            }
            if let Some(field_schema) = schema.fields.get(name) {
                let Some(value) = field_values.last() else {
                    continue;
                };
                if !self.validate_value(value, &field_schema.value, &field_path) {
                    return false;
                }
            } else if !schema.allow_unknown_fields
                && !self.record(
                    SchemaDiagnosticCode::UnknownObjectField,
                    "Object field is not allowed by the schema",
                    field_path,
                )
            {
                return false;
            }
        }
        true
    }

    fn validate_call(
        &mut self,
        name: &str,
        arguments: &[Value],
        schema: &CallSchema,
        path: &str,
    ) -> bool {
        if !schema.allowed_names.is_empty()
            && !schema.allowed_names.contains(name)
            && !self.record(
                SchemaDiagnosticCode::CallName,
                "Call function is not allowed by the schema",
                format!("{path}.function"),
            )
        {
            return false;
        }
        if !schema.arguments.contains(arguments.len())
            && !self.record(
                SchemaDiagnosticCode::CallArgumentCount,
                format!(
                    "Expected {}, found {}",
                    schema.arguments.describe("argument", "arguments"),
                    arguments.len()
                ),
                format!("{path}.arguments"),
            )
        {
            return false;
        }
        for (index, argument) in arguments.iter().enumerate() {
            if !self.validate_value(
                argument,
                &schema.argument,
                &format!("{path}.arguments[{index}]"),
            ) {
                return false;
            }
        }
        true
    }

    fn record(
        &mut self,
        code: SchemaDiagnosticCode,
        message: impl Into<String>,
        path: String,
    ) -> bool {
        if self.report.diagnostics.len() >= self.max_diagnostics {
            self.report.truncated = true;
            return false;
        }
        self.report.diagnostics.push(SchemaDiagnostic {
            code,
            message: message.into(),
            path,
        });
        true
    }
}

fn bare_attribute(block: &Block) -> Option<(&str, &Value)> {
    if block.labels.is_empty() && block.blocks.is_empty() && block.attributes.len() == 1 {
        return block
            .attributes
            .get(&block.name)
            .map(|value| (block.name.as_str(), value));
    }
    None
}

fn value_kind(value: &Value) -> &'static str {
    match value {
        Value::String(_) => "String",
        Value::Number(_) => "Number",
        Value::Bool(_) => "Bool",
        Value::List(_) => "List",
        Value::Object(_) => "Object",
        Value::Null => "Null",
        Value::Call(_, _) => "Call",
    }
}

fn value_matches(value: &Value, schema: &ValueSchema) -> bool {
    match schema.kind() {
        ValueSchemaKind::Any => true,
        ValueSchemaKind::String => matches!(value, Value::String(_)),
        ValueSchemaKind::Number => matches!(value, Value::Number(_)),
        ValueSchemaKind::Bool => matches!(value, Value::Bool(_)),
        ValueSchemaKind::Null => matches!(value, Value::Null),
        ValueSchemaKind::List(item_schema) => {
            matches!(value, Value::List(items) if items.iter().all(|item| value_matches(item, item_schema)))
        }
        ValueSchemaKind::Object(object_schema) => {
            matches!(value, Value::Object(fields) if object_matches(fields, object_schema))
        }
        ValueSchemaKind::Call(call_schema) => {
            matches!(value, Value::Call(name, arguments) if call_matches(name, arguments, call_schema))
        }
        ValueSchemaKind::OneOf(variants) => {
            variants.iter().any(|variant| value_matches(value, variant))
        }
    }
}

fn object_matches(fields: &[(String, Value)], schema: &ObjectSchema) -> bool {
    let mut values: BTreeMap<&str, Vec<&Value>> = BTreeMap::new();
    for (name, value) in fields {
        values.entry(name.as_str()).or_default().push(value);
    }
    if schema
        .fields
        .iter()
        .any(|(name, field)| field.required && !values.contains_key(name.as_str()))
    {
        return false;
    }
    values.into_iter().all(|(name, values)| {
        values.len() == 1
            && schema
                .fields
                .get(name)
                .map_or(schema.allow_unknown_fields, |field| {
                    value_matches(values[0], &field.value)
                })
    })
}

fn call_matches(name: &str, arguments: &[Value], schema: &CallSchema) -> bool {
    (schema.allowed_names.is_empty() || schema.allowed_names.contains(name))
        && schema.arguments.contains(arguments.len())
        && arguments
            .iter()
            .all(|argument| value_matches(argument, &schema.argument))
}
