use a3s_acl::{parse_with_limits, DiagnosticCode, ParseLimits};
use serde::Deserialize;

const CASES: &str = include_str!("../fixtures/diagnostics/cases.json");

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticCase {
    name: String,
    input: String,
    limits: LimitOverrides,
    expected: ExpectedDiagnostic,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LimitOverrides {
    max_document_bytes: Option<usize>,
    max_nesting_depth: Option<usize>,
    max_collection_items: Option<usize>,
    max_token_bytes: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct ExpectedDiagnostic {
    code: String,
    message: String,
    span: ExpectedSpan,
}

#[derive(Debug, Deserialize)]
struct ExpectedSpan {
    start: ExpectedLocation,
    end: ExpectedLocation,
}

#[derive(Debug, Deserialize)]
struct ExpectedLocation {
    line: usize,
    column: usize,
    offset: usize,
}

fn limits(overrides: &LimitOverrides) -> ParseLimits {
    let defaults = ParseLimits::default();
    ParseLimits {
        max_document_bytes: overrides
            .max_document_bytes
            .unwrap_or(defaults.max_document_bytes),
        max_nesting_depth: overrides
            .max_nesting_depth
            .unwrap_or(defaults.max_nesting_depth),
        max_collection_items: overrides
            .max_collection_items
            .unwrap_or(defaults.max_collection_items),
        max_token_bytes: overrides
            .max_token_bytes
            .unwrap_or(defaults.max_token_bytes),
    }
}

#[test]
fn shared_diagnostics_have_stable_codes_spans_and_redacted_messages() {
    let cases: Vec<DiagnosticCase> = serde_json::from_str(CASES).unwrap();

    for case in cases {
        let error = parse_with_limits(&case.input, limits(&case.limits)).unwrap_err();

        assert_eq!(
            error.code.as_str(),
            case.expected.code,
            "{} code",
            case.name
        );
        assert_eq!(
            error.message, case.expected.message,
            "{} message",
            case.name
        );
        assert_eq!(
            (
                error.span.start.line,
                error.span.start.column,
                error.span.start.offset,
                error.span.end.line,
                error.span.end.column,
                error.span.end.offset,
            ),
            (
                case.expected.span.start.line,
                case.expected.span.start.column,
                case.expected.span.start.offset,
                case.expected.span.end.line,
                case.expected.span.end.column,
                case.expected.span.end.offset,
            ),
            "{} span",
            case.name
        );
        assert_eq!(
            (error.line, error.column),
            (
                case.expected.span.start.line,
                case.expected.span.start.column
            ),
            "{} compatibility location",
            case.name
        );
        assert!(
            !error.message.contains("TOP_SECRET"),
            "{} must not echo source values",
            case.name
        );
    }

    assert_eq!(
        DiagnosticCode::UnexpectedToken.as_str(),
        "acl.parse.unexpected_token"
    );

    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<a3s_acl::ParseError>();
}
