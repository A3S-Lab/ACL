// ============================================================================
// ACL Lexer - Tokenizer for Agent Configuration Language
// ============================================================================

use std::str::FromStr;

/// A token produced by the lexer
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Punctuation
    LeftBrace,    // {
    RightBrace,   // }
    LeftBracket,  // [
    RightBracket, // ]
    LeftParen,    // (
    RightParen,   // )
    Equal,        // =
    PlusEqual,    // +=
    Colon,        // :
    Comma,        // ,
    Comment,      // # comment
    Newline,      // \n

    // Identifiers and literals
    Ident(String),  // foo_bar
    String(String), // "hello" or 'hello'
    Number(f64),    // 42 or 3.14

    // Keywords
    True,
    False,
    Null,

    // End of file
    Eof,
}

/// A location in the source code
#[derive(Debug, Clone, Copy, Default)]
pub struct Location {
    pub line: usize,
    pub column: usize,
    pub offset: usize,
}

/// A span of source code
#[derive(Debug, Clone, Default)]
pub struct Span {
    pub start: Location,
    pub end: Location,
}

/// A token with its source location
#[derive(Debug, Clone)]
pub struct TokenWithSpan {
    pub token: Token,
    pub span: Span,
}

impl TokenWithSpan {
    pub fn new(token: Token, start: Location, end: Location) -> Self {
        Self {
            token,
            span: Span { start, end },
        }
    }
}

/// Lexer state machine
pub struct Lexer<'a> {
    input: &'a str,
    chars: Vec<char>,
    pos: usize,
    pub location: Location,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        let chars: Vec<char> = input.chars().collect();
        Self {
            input,
            chars,
            pos: 0,
            location: Location {
                line: 1,
                column: 1,
                offset: 0,
            },
        }
    }

    fn current(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn advance(&mut self) {
        if self.pos < self.chars.len() {
            let c = self.chars[self.pos];
            self.pos += 1;
            self.location.offset += 1;
            if c == '\n' {
                self.location.line += 1;
                self.location.column = 1;
            } else {
                self.location.column += 1;
            }
        }
    }

    fn start_location(&self) -> Location {
        self.location
    }

    fn make_token(&self, token: Token, start: Location) -> TokenWithSpan {
        TokenWithSpan::new(token, start, self.location)
    }

    /// Tokenize the entire input
    pub fn tokenize(&mut self) -> Vec<TokenWithSpan> {
        let mut tokens = Vec::new();

        while let Some(c) = self.current() {
            let start = self.start_location();

            match c {
                '{' => {
                    self.advance();
                    tokens.push(self.make_token(Token::LeftBrace, start));
                }
                '}' => {
                    self.advance();
                    tokens.push(self.make_token(Token::RightBrace, start));
                }
                '[' => {
                    self.advance();
                    tokens.push(self.make_token(Token::LeftBracket, start));
                }
                ']' => {
                    self.advance();
                    tokens.push(self.make_token(Token::RightBracket, start));
                }
                '(' => {
                    self.advance();
                    tokens.push(self.make_token(Token::LeftParen, start));
                }
                ')' => {
                    self.advance();
                    tokens.push(self.make_token(Token::RightParen, start));
                }
                ',' => {
                    self.advance();
                    tokens.push(self.make_token(Token::Comma, start));
                }
                '=' => {
                    self.advance();
                    tokens.push(self.make_token(Token::Equal, start));
                }
                '+' => {
                    self.advance();
                    if self.current() == Some('=') {
                        self.advance();
                        tokens.push(self.make_token(Token::PlusEqual, start));
                    }
                }
                ':' => {
                    self.advance();
                    tokens.push(self.make_token(Token::Colon, start));
                }
                '"' => {
                    self.advance();
                    let s = self.read_string_double();
                    tokens.push(self.make_token(Token::String(s), start));
                }
                '\'' => {
                    self.advance();
                    let s = self.read_string_single();
                    tokens.push(self.make_token(Token::String(s), start));
                }
                '#' | '/' => {
                    self.advance();
                    let _comment = self.read_comment();
                    tokens.push(self.make_token(Token::Comment, start));
                    // Skip adding comment tokens, but track them
                }
                '\n' | '\r' => {
                    self.advance();
                    tokens.push(self.make_token(Token::Newline, start));
                }
                ' ' | '\t' => {
                    self.advance();
                    // Skip whitespace
                }
                _ if c.is_ascii_digit() || c == '-' || c == '.' => {
                    let start = self.start_location();
                    let (token, _) = self.read_number();
                    tokens.push(self.make_token(token, start));
                }
                _ if Self::is_ident_start(c) => {
                    let start = self.start_location();
                    let (token, _) = self.read_ident();
                    tokens.push(self.make_token(token, start));
                }
                _ => {
                    self.advance();
                }
            }
        }

        tokens.push(TokenWithSpan::new(Token::Eof, self.location, self.location));

        tokens
    }

    fn read_string_double(&mut self) -> String {
        let mut result = String::new();
        while let Some(c) = self.current() {
            match c {
                '"' => {
                    self.advance();
                    break;
                }
                '\\' => {
                    self.advance();
                    if let Some(escaped) = self.current() {
                        match escaped {
                            'n' => result.push('\n'),
                            'r' => result.push('\r'),
                            't' => result.push('\t'),
                            '\\' => result.push('\\'),
                            '"' => result.push('"'),
                            _ => {
                                result.push('\\');
                                result.push(escaped);
                            }
                        }
                        self.advance();
                    }
                }
                '\n' => break,
                _ => {
                    result.push(c);
                    self.advance();
                }
            }
        }
        result
    }

    fn read_string_single(&mut self) -> String {
        let mut result = String::new();
        while let Some(c) = self.current() {
            match c {
                '\'' => {
                    self.advance();
                    break;
                }
                '\\' => {
                    self.advance();
                    if let Some(escaped) = self.current() {
                        match escaped {
                            '\'' => result.push('\''),
                            _ => {
                                result.push('\\');
                                result.push(escaped);
                            }
                        }
                        self.advance();
                    }
                }
                '\n' => break,
                _ => {
                    result.push(c);
                    self.advance();
                }
            }
        }
        result
    }

    fn read_comment(&mut self) -> String {
        let mut result = String::new();
        while let Some(c) = self.current() {
            if c == '\n' {
                break;
            }
            result.push(c);
            self.advance();
        }
        result
    }

    fn read_number(&mut self) -> (Token, usize) {
        let start = self.pos;
        let mut has_dot = false;
        let mut has_e = false;

        // Handle negative
        if self.current() == Some('-') {
            self.advance();
        }

        while let Some(c) = self.current() {
            match c {
                '0'..='9' => {
                    self.advance();
                }
                '.' if !has_dot && !has_e => {
                    has_dot = true;
                    self.advance();
                }
                'e' | 'E' if !has_e => {
                    has_e = true;
                    self.advance();
                    if self.current() == Some('+') || self.current() == Some('-') {
                        self.advance();
                    }
                }
                _ => break,
            }
        }

        let num_str = &self.input[start..self.pos];
        let number = f64::from_str(num_str).unwrap_or(0.0);
        (Token::Number(number), start)
    }

    fn read_ident(&mut self) -> (Token, usize) {
        let start = self.pos;

        while let Some(c) = self.current() {
            if Self::is_ident_part(c) {
                self.advance();
            } else {
                break;
            }
        }

        let ident = &self.input[start..self.pos];
        let token = match ident {
            "true" => Token::True,
            "false" => Token::False,
            "null" => Token::Null,
            _ => Token::Ident(ident.to_string()),
        };

        (token, start)
    }

    fn is_ident_start(c: char) -> bool {
        c.is_alphabetic() || c == '_'
    }

    fn is_ident_part(c: char) -> bool {
        c.is_alphanumeric() || c == '_'
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer_basic() {
        let input = r#"
            name = "test"
            count = 42
            enabled = true
        "#;
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        let idents: Vec<_> = tokens
            .iter()
            .filter_map(|t| match &t.token {
                Token::Ident(s) => Some(s.clone()),
                _ => None,
            })
            .collect();
        assert!(idents.contains(&"name".to_string()));
        assert!(idents.contains(&"count".to_string()));
        assert!(idents.contains(&"enabled".to_string()));
    }

    #[test]
    fn test_lexer_numbers() {
        let input = "value = -3.14e+10";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        assert!(tokens.iter().any(|t| matches!(&t.token, Token::Number(_))));
    }

    #[test]
    fn test_lexer_keywords() {
        let input = "true false null";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        assert!(tokens.iter().any(|t| matches!(&t.token, Token::True)));
        assert!(tokens.iter().any(|t| matches!(&t.token, Token::False)));
        assert!(tokens.iter().any(|t| matches!(&t.token, Token::Null)));
    }

    #[test]
    fn test_lexer_string_double_quotes() {
        let input = r#""hello world""#;
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        let string_token = tokens.iter().find(|t| matches!(&t.token, Token::String(_)));
        assert!(string_token.is_some());
        if let Token::String(s) = &string_token.unwrap().token {
            assert_eq!(s, "hello world");
        }
    }

    #[test]
    fn test_lexer_string_single_quotes() {
        let input = "'hello world'";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        let string_token = tokens.iter().find(|t| matches!(&t.token, Token::String(_)));
        assert!(string_token.is_some());
        if let Token::String(s) = &string_token.unwrap().token {
            assert_eq!(s, "hello world");
        }
    }

    #[test]
    fn test_lexer_string_escapes() {
        let input = r#""hello\nworld\ttest\r""#;
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        let string_token = tokens.iter().find(|t| matches!(&t.token, Token::String(_)));
        assert!(string_token.is_some());
        if let Token::String(s) = &string_token.unwrap().token {
            assert_eq!(s, "hello\nworld\ttest\r");
        }
    }

    #[test]
    fn test_lexer_string_quote_escape() {
        let input = r#""hello\"world""#;
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        let string_token = tokens.iter().find(|t| matches!(&t.token, Token::String(_)));
        assert!(string_token.is_some());
        if let Token::String(s) = &string_token.unwrap().token {
            assert_eq!(s, "hello\"world");
        }
    }

    #[test]
    fn test_lexer_string_backslash_escape() {
        let input = r#""hello\\world""#;
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        let string_token = tokens.iter().find(|t| matches!(&t.token, Token::String(_)));
        assert!(string_token.is_some());
        if let Token::String(s) = &string_token.unwrap().token {
            assert_eq!(s, "hello\\world");
        }
    }

    #[test]
    fn test_lexer_ident_with_underscore() {
        let input = "my_variable";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        assert!(tokens
            .iter()
            .any(|t| t.token == Token::Ident("my_variable".to_string())));
    }

    #[test]
    fn test_lexer_plus_equal() {
        let input = "+=";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        assert!(tokens.iter().any(|t| matches!(&t.token, Token::PlusEqual)));
    }

    #[test]
    fn test_lexer_punctuation() {
        let input = "{}[]=:";
        let mut lexer = Lexer::new(input);
        let toks = lexer.tokenize();
        let tokens: Vec<_> = toks
            .iter()
            .take_while(|t| !matches!(&t.token, Token::Eof))
            .collect();

        assert_eq!(tokens.len(), 6);
        assert!(tokens.iter().any(|t| matches!(&t.token, Token::LeftBrace)));
        assert!(tokens.iter().any(|t| matches!(&t.token, Token::RightBrace)));
        assert!(tokens
            .iter()
            .any(|t| matches!(&t.token, Token::LeftBracket)));
        assert!(tokens
            .iter()
            .any(|t| matches!(&t.token, Token::RightBracket)));
        assert!(tokens.iter().any(|t| matches!(&t.token, Token::Equal)));
        assert!(tokens.iter().any(|t| matches!(&t.token, Token::Colon)));
    }

    #[test]
    fn test_lexer_negative_number() {
        let input = "value = -42";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        let num_token = tokens.iter().find(|t| matches!(&t.token, Token::Number(_)));
        assert!(num_token.is_some());
        if let Token::Number(n) = &num_token.unwrap().token {
            assert_eq!(*n, -42.0);
        }
    }

    #[test]
    fn test_lexer_float_number() {
        let input = "value = 3.14159";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        let num_token = tokens.iter().find(|t| matches!(&t.token, Token::Number(_)));
        assert!(num_token.is_some());
        if let Token::Number(n) = &num_token.unwrap().token {
            assert!((*n - 3.14159).abs() < 0.00001);
        }
    }

    #[test]
    fn test_lexer_scientific_notation() {
        let input = "value = 1e10";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        let num_token = tokens.iter().find(|t| matches!(&t.token, Token::Number(_)));
        assert!(num_token.is_some());
        if let Token::Number(n) = &num_token.unwrap().token {
            assert!((*n - 1e10).abs() < 1.0);
        }
    }

    #[test]
    fn test_lexer_location_tracking() {
        let input = "a\nb";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        let b_token = tokens.iter().find(|t| {
            matches!(&t.token, Token::Ident(_)) && t.token == Token::Ident("b".to_string())
        });
        assert!(b_token.is_some());
        assert_eq!(b_token.unwrap().span.start.line, 2);
        assert_eq!(b_token.unwrap().span.start.column, 1);
    }

    #[test]
    fn test_lexer_eof() {
        let input = "test";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        assert!(tokens
            .last()
            .map(|t| matches!(&t.token, Token::Eof))
            .unwrap_or(false));
    }

    #[test]
    fn test_lexer_empty_string() {
        let input = r#""""#;
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        let string_token = tokens.iter().find(|t| matches!(&t.token, Token::String(_)));
        assert!(string_token.is_some());
        if let Token::String(s) = &string_token.unwrap().token {
            assert_eq!(s, "");
        }
    }

    #[test]
    fn test_lexer_whitespace() {
        let input = "  \t    ";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        // Should only have EOF (space and tab are skipped, no newlines in this input)
        let non_eof: Vec<_> = tokens
            .iter()
            .filter(|t| !matches!(&t.token, Token::Eof))
            .collect();
        assert!(non_eof.is_empty());
    }

    #[test]
    fn test_lexer_slash_comment() {
        let input = "// this is a comment";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        // Comment tokens are tracked but the comment content is skipped
        // The comment token is still produced
        let comments: Vec<_> = tokens
            .iter()
            .filter(|t| matches!(&t.token, Token::Comment))
            .collect();
        assert_eq!(comments.len(), 1);
    }

    #[test]
    fn test_lexer_only_newlines() {
        let input = "\n\n\n";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        // Should have newlines and EOF
        let newlines: Vec<_> = tokens
            .iter()
            .filter(|t| matches!(&t.token, Token::Newline))
            .collect();
        assert_eq!(newlines.len(), 3);
    }

    #[test]
    fn test_lexer_carriage_return() {
        let input = "a\r\nb";
        let mut lexer = Lexer::new(input);
        let toks = lexer.tokenize();
        let idents: Vec<_> = toks
            .iter()
            .filter(|t| matches!(&t.token, Token::Ident(_)))
            .collect();
        assert_eq!(idents.len(), 2);
    }

    #[test]
    fn test_lexer_number_only_minus() {
        let input = "-";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        // Just a minus should produce a number token (value 0)
        let num_tokens: Vec<_> = tokens
            .iter()
            .filter(|t| matches!(&t.token, Token::Number(_)))
            .collect();
        assert_eq!(num_tokens.len(), 1);
    }

    #[test]
    fn test_lexer_plus_only() {
        let input = "+";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        // Plus alone should be skipped or produce ident
        let non_eof: Vec<_> = tokens
            .iter()
            .filter(|t| !matches!(&t.token, Token::Eof))
            .collect();
        assert!(non_eof.is_empty());
    }

    #[test]
    fn test_lexer_single_quote_string() {
        // Test single-quoted string with escaped single quote
        let input = "'hello\\'world'";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        let string_token = tokens.iter().find(|t| matches!(&t.token, Token::String(_)));
        assert!(string_token.is_some());
        if let Token::String(s) = &string_token.unwrap().token {
            assert_eq!(s, "hello'world");
        }
    }

    #[test]
    fn test_lexer_single_quote_string_unknown_escape() {
        // Test single-quoted string with unknown escape sequence
        let input = "'hello\\nworld'";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        let string_token = tokens.iter().find(|t| matches!(&t.token, Token::String(_)));
        assert!(string_token.is_some());
        // Single-quoted strings preserve escape sequences literally
        if let Token::String(s) = &string_token.unwrap().token {
            assert_eq!(s, "hello\\nworld");
        }
    }

    #[test]
    fn test_lexer_single_quote_with_newline() {
        // Test single-quoted string terminated by newline
        let input = "'hello
world'";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        let string_token = tokens.iter().find(|t| matches!(&t.token, Token::String(_)));
        assert!(string_token.is_some());
        if let Token::String(s) = &string_token.unwrap().token {
            assert_eq!(s, "hello");
        }
    }

    #[test]
    fn test_lexer_double_quote_string_unknown_escape() {
        // Test double-quoted string with unknown escape sequence
        let input = "\"hello\\qworld\"";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        let string_token = tokens.iter().find(|t| matches!(&t.token, Token::String(_)));
        assert!(string_token.is_some());
        if let Token::String(s) = &string_token.unwrap().token {
            assert_eq!(s, "hello\\qworld");
        }
    }
}
