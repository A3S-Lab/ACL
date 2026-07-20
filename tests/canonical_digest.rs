use a3s_acl::{
    canonical_bytes, canonical_digest, parse, CanonicalError, CANONICAL_DIGEST_ALGORITHM,
};
use serde::Deserialize;

const DIGEST_CASES: &str = include_str!("../fixtures/canonical/digest-cases.json");

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DigestCase {
    name: String,
    input: String,
    equivalent_input: String,
    canonical: String,
    digest: String,
}

fn digest_cases() -> Vec<DigestCase> {
    serde_json::from_str(DIGEST_CASES).expect("parse shared canonical digest cases")
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
