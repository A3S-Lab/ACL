use crate::lexer::{Location, Span};

/// Stable machine-readable category for one ACL parse diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticCode {
    DocumentBytesLimit,
    TokenBytesLimit,
    NestingDepthLimit,
    CollectionItemsLimit,
    UnexpectedToken,
    ExpectedToken,
    UnexpectedEof,
}

impl DiagnosticCode {
    /// Return the cross-SDK wire representation of this diagnostic code.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DocumentBytesLimit => "acl.limit.document_bytes",
            Self::TokenBytesLimit => "acl.limit.token_bytes",
            Self::NestingDepthLimit => "acl.limit.nesting_depth",
            Self::CollectionItemsLimit => "acl.limit.collection_items",
            Self::UnexpectedToken => "acl.parse.unexpected_token",
            Self::ExpectedToken => "acl.parse.expected_token",
            Self::UnexpectedEof => "acl.parse.unexpected_eof",
        }
    }

    /// Return true for resource-limit diagnostics that stop parsing immediately.
    pub const fn is_limit(self) -> bool {
        matches!(
            self,
            Self::DocumentBytesLimit
                | Self::TokenBytesLimit
                | Self::NestingDepthLimit
                | Self::CollectionItemsLimit
        )
    }
}

impl std::fmt::Display for DiagnosticCode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Structured ACL parse error with a stable code and byte-accurate source span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub code: DiagnosticCode,
    pub message: String,
    pub span: Span,
    /// Compatibility alias for `span.start.line`.
    pub line: usize,
    /// Compatibility alias for `span.start.column`.
    pub column: usize,
}

/// Bounded collection of ACL parse diagnostics.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiagnosticReport {
    /// Diagnostics in deterministic source order.
    pub diagnostics: Vec<ParseError>,
    /// Whether at least one additional diagnostic was observed after the configured budget.
    pub truncated: bool,
}

impl DiagnosticReport {
    /// Return true when no diagnostic was collected and the report was not truncated.
    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty() && !self.truncated
    }
}

impl ParseError {
    pub fn new(code: DiagnosticCode, message: impl Into<String>, span: Span) -> Self {
        Self {
            code,
            message: message.into(),
            line: span.start.line,
            column: span.start.column,
            span,
        }
    }

    pub(crate) fn at(code: DiagnosticCode, message: impl Into<String>, location: Location) -> Self {
        Self::new(code, message, Span::point(location))
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "Parse error at line {}, column {}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for ParseError {}
