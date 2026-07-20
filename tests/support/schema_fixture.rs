use a3s_acl::{
    AttributeSchema, BlockSchema, CallSchema, Cardinality, ObjectSchema, Schema, ValueSchema,
};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FixtureSchema {
    #[serde(default)]
    attributes: BTreeMap<String, FixtureAttribute>,
    #[serde(default)]
    blocks: BTreeMap<String, FixtureBlock>,
    #[serde(default)]
    allow_unknown_attributes: bool,
    #[serde(default)]
    allow_unknown_blocks: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureAttribute {
    #[serde(default)]
    required: bool,
    value: FixtureValue,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureBlock {
    occurrences: FixtureCardinality,
    labels: FixtureCardinality,
    body: FixtureSchema,
    #[serde(default)]
    unordered: bool,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureCardinality {
    min: usize,
    max: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
enum FixtureValue {
    Any,
    String,
    Number,
    Bool,
    Null,
    List {
        item: Box<FixtureValue>,
    },
    Object {
        #[serde(default)]
        fields: BTreeMap<String, FixtureAttribute>,
        #[serde(default, rename = "allowUnknownFields")]
        allow_unknown_fields: bool,
    },
    Call {
        #[serde(default)]
        names: Vec<String>,
        arguments: FixtureCardinality,
        argument: Box<FixtureValue>,
    },
    OneOf {
        variants: Vec<FixtureValue>,
    },
}

pub fn schema(fixture: FixtureSchema) -> Schema {
    let mut schema = Schema::new()
        .allow_unknown_attributes(fixture.allow_unknown_attributes)
        .allow_unknown_blocks(fixture.allow_unknown_blocks);
    for (name, attribute) in fixture.attributes {
        schema = schema.attribute(name, attribute_schema(attribute));
    }
    for (name, block) in fixture.blocks {
        schema = schema.block(name, block_schema(block));
    }
    schema
}

fn attribute_schema(fixture: FixtureAttribute) -> AttributeSchema {
    let value = value_schema(fixture.value);
    if fixture.required {
        AttributeSchema::required(value)
    } else {
        AttributeSchema::optional(value)
    }
}

fn block_schema(fixture: FixtureBlock) -> BlockSchema {
    BlockSchema::new(schema(fixture.body))
        .occurrences(cardinality(fixture.occurrences))
        .labels(cardinality(fixture.labels))
        .unordered(fixture.unordered)
}

fn cardinality(fixture: FixtureCardinality) -> Cardinality {
    Cardinality::new(fixture.min, fixture.max).expect("fixture cardinality is valid")
}

fn value_schema(fixture: FixtureValue) -> ValueSchema {
    match fixture {
        FixtureValue::Any => ValueSchema::any(),
        FixtureValue::String => ValueSchema::string(),
        FixtureValue::Number => ValueSchema::number(),
        FixtureValue::Bool => ValueSchema::bool(),
        FixtureValue::Null => ValueSchema::null(),
        FixtureValue::List { item } => ValueSchema::list(value_schema(*item)),
        FixtureValue::Object {
            fields,
            allow_unknown_fields,
        } => {
            let mut schema = ObjectSchema::new().allow_unknown_fields(allow_unknown_fields);
            for (name, field) in fields {
                schema = schema.field(name, attribute_schema(field));
            }
            ValueSchema::object(schema)
        }
        FixtureValue::Call {
            names,
            arguments,
            argument,
        } => {
            let mut schema =
                CallSchema::new().arguments(cardinality(arguments), value_schema(*argument));
            for name in names {
                schema = schema.allowed_name(name);
            }
            ValueSchema::call(schema)
        }
        FixtureValue::OneOf { variants } => {
            ValueSchema::one_of(variants.into_iter().map(value_schema).collect::<Vec<_>>())
                .expect("fixture union is non-empty")
        }
    }
}
