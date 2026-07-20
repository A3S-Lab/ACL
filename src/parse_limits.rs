use crate::lexer::{Token, TokenWithSpan};
use crate::parser::ParseError;

/// Resource limits applied by [`crate::parse`] and [`crate::parse_with_limits`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseLimits {
    /// Maximum UTF-8 byte length of one ACL document.
    pub max_document_bytes: usize,
    /// Maximum number of nested block, list, object, and call delimiters.
    pub max_nesting_depth: usize,
    /// Maximum number of entries in one document, block body, label list, value collection, or
    /// function argument list.
    pub max_collection_items: usize,
    /// Maximum UTF-8 byte length of one source token.
    pub max_token_bytes: usize,
}

/// Default resource limits used by [`crate::parse`].
pub const DEFAULT_PARSE_LIMITS: ParseLimits = ParseLimits {
    max_document_bytes: 1024 * 1024,
    max_nesting_depth: 64,
    max_collection_items: 10_000,
    max_token_bytes: 256 * 1024,
};

impl Default for ParseLimits {
    fn default() -> Self {
        DEFAULT_PARSE_LIMITS
    }
}

pub(crate) fn nesting_limit_error(
    tokens: &[TokenWithSpan],
    max_nesting_depth: usize,
) -> Option<ParseError> {
    let mut depth = 0usize;
    for token in tokens {
        match token.token {
            Token::LeftBrace | Token::LeftBracket | Token::LeftParen => {
                depth = depth.saturating_add(1);
                if depth > max_nesting_depth {
                    return Some(ParseError {
                        message: format!(
                            "ACL parse limit exceeded: nesting depth is greater than {max_nesting_depth}"
                        ),
                        line: token.span.start.line,
                        column: token.span.start.column,
                    });
                }
            }
            Token::RightBrace | Token::RightBracket | Token::RightParen => {
                depth = depth.saturating_sub(1);
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parse, parse_with_limits};

    const COLLECTION_LIMIT_FIXTURE: &str = include_str!("../fixtures/limits/collection.acl");
    const NESTING_LIMIT_FIXTURE: &str = include_str!("../fixtures/limits/nested.acl");
    const TOKEN_LIMIT_FIXTURE: &str = include_str!("../fixtures/limits/token.acl");

    fn limits() -> ParseLimits {
        ParseLimits::default()
    }

    #[test]
    fn rejects_oversized_documents_before_parsing() {
        let error = parse_with_limits(
            TOKEN_LIMIT_FIXTURE,
            ParseLimits {
                max_document_bytes: TOKEN_LIMIT_FIXTURE.len() - 1,
                ..limits()
            },
        )
        .unwrap_err();

        assert_eq!(
            error.message,
            format!(
                "ACL parse limit exceeded: document is larger than {} bytes",
                TOKEN_LIMIT_FIXTURE.len() - 1
            )
        );
        assert_eq!((error.line, error.column), (1, 1));

        let unicode_input = r#"name = "智谱""#;
        assert!(unicode_input.len() > unicode_input.chars().count());
        assert!(parse_with_limits(
            unicode_input,
            ParseLimits {
                max_document_bytes: unicode_input.len() - 1,
                ..limits()
            },
        )
        .is_err());
    }

    #[test]
    fn rejects_source_tokens_by_utf8_byte_length() {
        let error = parse_with_limits(
            TOKEN_LIMIT_FIXTURE,
            ParseLimits {
                max_token_bytes: 8,
                ..limits()
            },
        )
        .unwrap_err();

        assert_eq!(
            error.message,
            "ACL parse limit exceeded: token is longer than 8 bytes"
        );
        assert_eq!((error.line, error.column), (1, 8));

        let unicode_error = parse_with_limits(
            r#"name = "智谱""#,
            ParseLimits {
                max_token_bytes: 7,
                ..limits()
            },
        )
        .unwrap_err();
        assert_eq!(
            unicode_error.message,
            "ACL parse limit exceeded: token is longer than 7 bytes"
        );
    }

    #[test]
    fn rejects_excessive_structural_nesting() {
        let error = parse_with_limits(
            NESTING_LIMIT_FIXTURE,
            ParseLimits {
                max_nesting_depth: 1,
                ..limits()
            },
        )
        .unwrap_err();

        assert_eq!(
            error.message,
            "ACL parse limit exceeded: nesting depth is greater than 1"
        );
        assert_eq!((error.line, error.column), (2, 9));
    }

    #[test]
    fn bounds_implicit_block_recursion() {
        let error = parse_with_limits(
            "first second third fourth",
            ParseLimits {
                max_nesting_depth: 1,
                ..limits()
            },
        )
        .unwrap_err();

        assert_eq!(
            error.message,
            "ACL parse limit exceeded: nesting depth is greater than 1"
        );
    }

    #[test]
    fn rejects_oversized_collections() {
        let error = parse_with_limits(
            COLLECTION_LIMIT_FIXTURE,
            ParseLimits {
                max_collection_items: 2,
                ..limits()
            },
        )
        .unwrap_err();

        assert_eq!(
            error.message,
            "ACL parse limit exceeded: collection has more than 2 items"
        );
        assert_eq!(error.line, 1);
    }

    #[test]
    fn applies_collection_limits_to_every_public_collection_shape() {
        for input in [
            "first = 1\nsecond = 2\nthird = 3",
            "root { first = 1 second = 2 third = 3 }",
            r#"root "first" "second" "third" { value = true }"#,
            "value = { first = 1 second = 2 third = 3 }",
            "value = concat(1, 2, 3)",
        ] {
            let error = parse_with_limits(
                input,
                ParseLimits {
                    max_collection_items: 2,
                    ..limits()
                },
            )
            .unwrap_err();

            assert_eq!(
                error.message, "ACL parse limit exceeded: collection has more than 2 items",
                "input should be limited: {input}"
            );
        }
    }

    #[test]
    fn default_limits_accept_the_shared_fixtures() {
        assert_eq!(ParseLimits::default(), DEFAULT_PARSE_LIMITS);
        parse(TOKEN_LIMIT_FIXTURE).unwrap();
        parse(NESTING_LIMIT_FIXTURE).unwrap();
        parse(COLLECTION_LIMIT_FIXTURE).unwrap();
    }
}
