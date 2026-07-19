use a3s_acl::{generate, parse, Value};

const STRING_VALUES: &str = include_str!("../fixtures/canonical/string-values.acl");

fn canonical_fixture() -> &'static str {
    STRING_VALUES.strip_suffix('\n').unwrap_or(STRING_VALUES)
}

#[test]
fn rust_preserves_every_canonical_string_value() {
    let expected = canonical_fixture();
    let document = parse(expected).expect("parse shared canonical string fixture");

    assert_eq!(generate(&document), expected);

    let strings = document.blocks.first().expect("strings block");
    assert_eq!(strings.attributes.len(), 8);
    for (name, value) in &strings.attributes {
        assert!(
            matches!(value, Value::String(_)),
            "{name} changed from a string to {value:?}"
        );
    }
}

#[test]
fn rust_empty_string_round_trip_is_type_stable() {
    let document = parse("empty = \"\"").expect("parse empty string");
    let generated = generate(&document);

    assert_eq!(generated, "empty = \"\"\n");
    let reparsed = parse(&generated).expect("reparse generated empty string");
    let value = reparsed.blocks[0]
        .attributes
        .get("empty")
        .expect("empty string attribute");
    assert!(matches!(value, Value::String(value) if value.is_empty()));
}
