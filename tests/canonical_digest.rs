#[path = "support/schema_fixture.rs"]
mod schema_fixture;

use a3s_acl::{
    canonical_bytes, canonical_bytes_with_schema, canonical_digest, canonical_digest_with_schema,
    parse, validate_document, CanonicalError, CANONICAL_DIGEST_ALGORITHM,
};
use schema_fixture::{schema, FixtureSchema};
use serde::Deserialize;

const DIGEST_CASES: &str = include_str!("../fixtures/canonical/digest-cases.json");
const SCHEMA_ORDER_CASES: &str =
    include_str!("../fixtures/canonical/schema-block-order-cases.json");

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DigestCase {
    name: String,
    input: String,
    equivalent_input: String,
    canonical: String,
    digest: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SchemaOrderFixture {
    schema: FixtureSchema,
    adversarial_count: usize,
    equivalent_cases: Vec<EquivalentSchemaOrderCase>,
    ordered_cases: Vec<OrderedSchemaOrderCase>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EquivalentSchemaOrderCase {
    name: String,
    input: String,
    equivalent_input: String,
    canonical: String,
    digest: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrderedSchemaOrderCase {
    name: String,
    first_input: String,
    second_input: String,
}

fn digest_cases() -> Vec<DigestCase> {
    serde_json::from_str(DIGEST_CASES).expect("parse shared canonical digest cases")
}

fn providers(indexes: impl Iterator<Item = usize>) -> String {
    indexes
        .map(|index| format!("provider \"{index:04}\" {{ enabled = true }}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn rust_matches_shared_canonical_bytes_and_digests() {
    assert_eq!(CANONICAL_DIGEST_ALGORITHM, "sha256");

    for case in digest_cases() {
        for input in [&case.input, &case.equivalent_input] {
            let document = parse(input).unwrap_or_else(|error| {
                panic!("parse canonical digest case {:?}: {error}", case.name)
            });
            let bytes = canonical_bytes(&document).unwrap_or_else(|error| {
                panic!("canonicalize digest case {:?}: {error}", case.name)
            });

            assert_eq!(
                std::str::from_utf8(&bytes).expect("canonical ACL is UTF-8"),
                case.canonical,
                "canonical bytes changed for {:?}",
                case.name
            );
            assert_eq!(
                canonical_digest(&document).expect("digest canonical ACL"),
                case.digest,
                "canonical digest changed for {:?}",
                case.name
            );
        }
    }
}

#[test]
fn canonical_digest_preserves_ordered_list_semantics() {
    let first = parse("value = [1, 2]").expect("parse first list");
    let second = parse("value = [2, 1]").expect("parse second list");

    assert_ne!(
        canonical_digest(&first).expect("digest first list"),
        canonical_digest(&second).expect("digest second list")
    );
}

#[test]
fn canonical_bytes_preserve_attributes_and_single_attribute_blocks() {
    let document = parse(
        r#"
enabled = true
limits {
  mode = "strict"
}
"#,
    )
    .expect("parse attribute and block");
    let canonical = canonical_bytes(&document).expect("canonicalize attribute and block");

    assert_eq!(
        std::str::from_utf8(&canonical).expect("canonical ACL is UTF-8"),
        "enabled = true\nlimits {\n  mode = \"strict\"\n}\n"
    );

    let reparsed =
        parse(std::str::from_utf8(&canonical).expect("canonical ACL is UTF-8")).expect("reparse");
    assert_eq!(
        canonical_bytes(&reparsed).expect("canonicalize reparsed document"),
        canonical,
        "canonical bytes must be idempotent"
    );
}

#[test]
fn canonical_digest_preserves_unicode_scalar_sequences() {
    let composed = parse("value = \"é\"").expect("parse composed string");
    let decomposed = parse("value = \"e\u{301}\"").expect("parse decomposed string");

    assert_ne!(
        canonical_digest(&composed).expect("digest composed string"),
        canonical_digest(&decomposed).expect("digest decomposed string")
    );
}

#[test]
fn canonical_digest_rejects_non_finite_programmatic_numbers() {
    let mut document = parse("value = 1").expect("parse finite document");
    *document.blocks[0]
        .attributes
        .get_mut("value")
        .expect("value attribute") = a3s_acl::Value::Number(f64::INFINITY);

    assert_eq!(
        canonical_bytes(&document),
        Err(CanonicalError::NonFiniteNumber)
    );
    assert_eq!(
        canonical_digest(&document),
        Err(CanonicalError::NonFiniteNumber)
    );
}

#[test]
fn schema_normalization_matches_shared_unordered_block_cases() {
    let fixture: SchemaOrderFixture =
        serde_json::from_str(SCHEMA_ORDER_CASES).expect("parse schema block order cases");
    let schema = schema(fixture.schema);

    for case in fixture.equivalent_cases {
        let source = parse(&case.input)
            .unwrap_or_else(|error| panic!("parse schema order case {:?}: {error}", case.name));
        let equivalent = parse(&case.equivalent_input)
            .unwrap_or_else(|error| panic!("parse schema order case {:?}: {error}", case.name));
        assert_ne!(
            canonical_digest(&source).expect("digest ordered source document"),
            canonical_digest(&equivalent).expect("digest ordered equivalent document"),
            "base canonical APIs must preserve block order for {:?}",
            case.name
        );

        for input in [&case.input, &case.equivalent_input] {
            let document = parse(input)
                .unwrap_or_else(|error| panic!("parse schema order case {:?}: {error}", case.name));
            assert!(
                validate_document(&document, &schema).is_empty(),
                "schema order case {:?} must pass admission",
                case.name
            );
            let bytes = canonical_bytes_with_schema(&document, &schema).unwrap_or_else(|error| {
                panic!("canonicalize schema order case {:?}: {error}", case.name)
            });
            assert_eq!(
                std::str::from_utf8(&bytes).expect("schema canonical ACL is UTF-8"),
                case.canonical,
                "schema canonical bytes changed for {:?}",
                case.name
            );
            assert_eq!(
                canonical_digest_with_schema(&document, &schema)
                    .expect("digest schema-normalized ACL"),
                case.digest,
                "schema canonical digest changed for {:?}",
                case.name
            );
        }
    }
}

#[test]
fn unmarked_and_unknown_blocks_remain_ordered() {
    let fixture: SchemaOrderFixture =
        serde_json::from_str(SCHEMA_ORDER_CASES).expect("parse schema block order cases");
    let schema = schema(fixture.schema);

    for case in fixture.ordered_cases {
        let first = parse(&case.first_input)
            .unwrap_or_else(|error| panic!("parse ordered case {:?}: {error}", case.name));
        let second = parse(&case.second_input)
            .unwrap_or_else(|error| panic!("parse ordered case {:?}: {error}", case.name));
        assert_ne!(
            canonical_digest_with_schema(&first, &schema).expect("digest first ordered document"),
            canonical_digest_with_schema(&second, &schema).expect("digest second ordered document"),
            "ordered case {:?} must remain order-sensitive",
            case.name
        );
    }
}

#[test]
fn schema_normalization_handles_adversarial_repetition_deterministically() {
    let fixture: SchemaOrderFixture =
        serde_json::from_str(SCHEMA_ORDER_CASES).expect("parse schema block order cases");
    let count = fixture.adversarial_count;
    let schema = schema(fixture.schema);
    let ascending = format!("version = 1\n{}", providers(0..count));
    let descending = format!("version = 1\n{}", providers((0..count).rev()));
    let ascending = parse(&ascending).expect("parse ascending adversarial providers");
    let descending = parse(&descending).expect("parse descending adversarial providers");

    assert_eq!(
        canonical_digest_with_schema(&ascending, &schema)
            .expect("digest ascending adversarial providers"),
        canonical_digest_with_schema(&descending, &schema)
            .expect("digest descending adversarial providers")
    );
}
