use a3s_acl::{
    parse, validate_document, validate_document_with_limits, AttributeSchema, BlockSchema,
    CallSchema, Cardinality, Document, ObjectSchema, ParseLimits, Schema, SchemaDiagnostic,
    SchemaReport, Value, ValueSchema,
};
use serde::Deserialize;
use std::collections::BTreeMap;

const FIXTURE: &str = include_str!("../fixtures/schema/admission-cases.json");

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Fixture {
    schema: FixtureSchema,
    cases: Vec<FixtureCase>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureSchema {
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
struct FixtureAttribute {
    #[serde(default)]
    required: bool,
    value: FixtureValue,
}

#[derive(Debug, Deserialize)]
struct FixtureBlock {
    occurrences: FixtureCardinality,
    labels: FixtureCardinality,
    body: FixtureSchema,
}

#[derive(Debug, Clone, Copy, Deserialize)]
struct FixtureCardinality {
    min: usize,
    max: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind")]
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureCase {
    name: String,
    input: String,
    limits: FixtureLimits,
    expected: FixtureReport,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureLimits {
    max_diagnostics: usize,
}

#[derive(Debug, Deserialize)]
struct FixtureReport {
    diagnostics: Vec<FixtureDiagnostic>,
    truncated: bool,
}

#[derive(Debug, Deserialize)]
struct FixtureDiagnostic {
    code: String,
    message: String,
    path: String,
}

fn schema(fixture: FixtureSchema) -> Schema {
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
}

fn cardinality(fixture: FixtureCardinality) -> Cardinality {
    Cardinality::new(fixture.min, fixture.max).unwrap()
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
            ValueSchema::one_of(variants.into_iter().map(value_schema).collect::<Vec<_>>()).unwrap()
        }
    }
}

fn assert_report(actual: &SchemaReport, expected: &FixtureReport, name: &str) {
    assert_eq!(actual.truncated, expected.truncated, "{name} truncation");
    assert_eq!(
        actual.diagnostics.len(),
        expected.diagnostics.len(),
        "{name} diagnostic count"
    );
    for (index, (actual, expected)) in actual
        .diagnostics
        .iter()
        .zip(&expected.diagnostics)
        .enumerate()
    {
        assert_eq!(
            actual.code.as_str(),
            expected.code,
            "{name} diagnostic {index} code"
        );
        assert_eq!(
            actual.message, expected.message,
            "{name} diagnostic {index} message"
        );
        assert_eq!(actual.path, expected.path, "{name} diagnostic {index} path");
        assert!(
            !actual.message.contains("TOP_SECRET") && !actual.path.contains("TOP_SECRET"),
            "{name} diagnostic {index} must not echo values or labels"
        );
    }
}

#[test]
fn rust_matches_shared_schema_admission_fixture() {
    let fixture: Fixture = serde_json::from_str(FIXTURE).unwrap();
    let schema = schema(fixture.schema);

    for case in fixture.cases {
        let document = parse(&case.input).unwrap();
        let report = validate_document_with_limits(
            &document,
            &schema,
            ParseLimits {
                max_diagnostics: case.limits.max_diagnostics,
                ..ParseLimits::default()
            },
        );
        assert_report(&report, &case.expected, &case.name);
    }

    let exact_budget_document = parse("version = \"TOP_SECRET\"\nextra = \"TOP_SECRET\"").unwrap();
    let exact_budget_report = validate_document_with_limits(
        &exact_budget_document,
        &schema,
        ParseLimits {
            max_diagnostics: 3,
            ..ParseLimits::default()
        },
    );
    assert_eq!(exact_budget_report.diagnostics.len(), 3);
    assert!(!exact_budget_report.truncated);

    let adversarial_input = (0..1_000)
        .map(|index| format!("unknown_{index} = \"TOP_SECRET\""))
        .collect::<Vec<_>>()
        .join("\n");
    let adversarial_document = parse(&adversarial_input).unwrap();
    let adversarial_report = validate_document_with_limits(
        &adversarial_document,
        &schema,
        ParseLimits {
            max_diagnostics: 3,
            ..ParseLimits::default()
        },
    );
    assert_eq!(adversarial_report.diagnostics.len(), 3);
    assert!(adversarial_report.truncated);

    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Schema>();
    assert_send_sync::<SchemaDiagnostic>();
    assert_send_sync::<SchemaReport>();
}

#[test]
fn schema_builders_reject_invalid_ranges_and_empty_unions() {
    assert!(Cardinality::new(2, Some(1)).is_err());
    assert!(ValueSchema::one_of(Vec::new()).is_err());
}

#[test]
fn open_schema_flags_accept_extension_points() {
    let document = parse("extension = \"value\"\ncustom \"label\" {\n  nested = true\n}").unwrap();
    let schema = Schema::new()
        .allow_unknown_attributes(true)
        .allow_unknown_blocks(true);
    assert!(validate_document(&document, &schema).is_empty());
}

#[test]
fn programmatic_object_duplicates_are_rejected() {
    let document = Document {
        blocks: vec![a3s_acl::Block {
            name: "payload".to_string(),
            labels: Vec::new(),
            blocks: Vec::new(),
            attributes: [(
                "payload".to_string(),
                Value::Object(vec![
                    ("owner".to_string(), Value::String("first".to_string())),
                    ("owner".to_string(), Value::String("TOP_SECRET".to_string())),
                ]),
            )]
            .into_iter()
            .collect(),
        }],
    };
    let schema = Schema::new().attribute(
        "payload",
        AttributeSchema::required(ValueSchema::object(
            ObjectSchema::new().field("owner", AttributeSchema::required(ValueSchema::string())),
        )),
    );
    let report = validate_document(&document, &schema);
    assert_eq!(report.diagnostics.len(), 1);
    assert_eq!(
        report.diagnostics[0].code.as_str(),
        "acl.schema.duplicate_object_field"
    );
    assert_eq!(
        report.diagnostics[0].path,
        "$.attributes.payload.fields.owner"
    );
    assert!(!report.diagnostics[0].message.contains("TOP_SECRET"));
}
