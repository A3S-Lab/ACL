use a3s_acl::{
    collect_diagnostics, collect_diagnostics_with_limits, parse_with_limits, DiagnosticCode,
    ParseError, ParseLimits,
};
use serde::Deserialize;

const CASES: &str = include_str!("../fixtures/diagnostics/cases.json");
const MULTI_CASES: &str = include_str!("../fixtures/diagnostics/multi-cases.json");

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticCase {
    name: String,
    input: String,
    limits: LimitOverrides,
    expected: ExpectedDiagnostic,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MultiDiagnosticCase {
    name: String,
    input: String,
    limits: LimitOverrides,
    expected: ExpectedDiagnosticReport,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LimitOverrides {
    max_document_bytes: Option<usize>,
    max_nesting_depth: Option<usize>,
    max_collection_items: Option<usize>,
    max_token_bytes: Option<usize>,
    max_diagnostics: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct ExpectedDiagnostic {
    code: String,
    message: String,
    span: ExpectedSpan,
}

#[derive(Debug, Deserialize)]
struct ExpectedDiagnosticReport {
    diagnostics: Vec<ExpectedDiagnostic>,
    truncated: bool,
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
        max_diagnostics: overrides
            .max_diagnostics
            .unwrap_or(defaults.max_diagnostics),
    }
}

fn assert_diagnostic(error: &ParseError, expected: &ExpectedDiagnostic, name: &str) {
    assert_eq!(error.code.as_str(), expected.code, "{name} code");
    assert_eq!(error.message, expected.message, "{name} message");
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
            expected.span.start.line,
            expected.span.start.column,
            expected.span.start.offset,
            expected.span.end.line,
            expected.span.end.column,
            expected.span.end.offset,
        ),
        "{name} span"
    );
    assert_eq!(
        (error.line, error.column),
        (expected.span.start.line, expected.span.start.column),
        "{name} compatibility location"
    );
    assert!(
        !error.message.contains("TOP_SECRET"),
        "{name} must not echo source values"
    );
}

#[test]
fn shared_diagnostics_have_stable_codes_spans_and_redacted_messages() {
    let cases: Vec<DiagnosticCase> = serde_json::from_str(CASES).unwrap();

    for case in cases {
        let error = parse_with_limits(&case.input, limits(&case.limits)).unwrap_err();
        assert_diagnostic(&error, &case.expected, &case.name);
    }

    assert_eq!(
        DiagnosticCode::UnexpectedToken.as_str(),
        "acl.parse.unexpected_token"
    );

    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<a3s_acl::ParseError>();
}

#[test]
fn shared_multi_diagnostics_are_bounded_and_stable() {
    let cases: Vec<MultiDiagnosticCase> = serde_json::from_str(MULTI_CASES).unwrap();

    for case in cases {
        let report = collect_diagnostics_with_limits(&case.input, limits(&case.limits));
        assert_eq!(
            report.truncated, case.expected.truncated,
            "{} truncation",
            case.name
        );
        assert_eq!(
            report.diagnostics.len(),
            case.expected.diagnostics.len(),
            "{} count",
            case.name
        );
        for (index, (error, expected)) in report
            .diagnostics
            .iter()
            .zip(&case.expected.diagnostics)
            .enumerate()
        {
            assert_diagnostic(
                error,
                expected,
                &format!("{} diagnostic {index}", case.name),
            );
        }
    }

    let adversarial_input = (0..1_000)
        .map(|index| format!("invalid_{index} = ]"))
        .collect::<Vec<_>>()
        .join("\n");
    let report = collect_diagnostics_with_limits(
        &adversarial_input,
        ParseLimits {
            max_diagnostics: 3,
            ..ParseLimits::default()
        },
    );
    assert_eq!(report.diagnostics.len(), 3);
    assert!(report.truncated);

    let exact_budget_report = collect_diagnostics_with_limits(
        "first = ]\nsecond = ]\nthird = ]",
        ParseLimits {
            max_diagnostics: 3,
            ..ParseLimits::default()
        },
    );
    assert_eq!(exact_budget_report.diagnostics.len(), 3);
    assert!(!exact_budget_report.truncated);
    assert!(collect_diagnostics("valid = true").is_empty());

    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<a3s_acl::DiagnosticReport>();
}
