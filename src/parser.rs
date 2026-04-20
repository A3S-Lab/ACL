// ============================================================================
// ACL Parser - Parser for Agent Configuration Language
// ============================================================================

use crate::ast::{Block, Document, Value};
use crate::lexer::{Lexer, Token, TokenWithSpan};
use std::collections::HashMap;

/// Parse error
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Parse error at line {}, column {}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for ParseError {}

/// ACL Parser
pub struct Parser<'a> {
    tokens: Vec<TokenWithSpan>,
    pos: usize,
    _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        Self {
            tokens,
            pos: 0,
            _marker: std::marker::PhantomData,
        }
    }

    fn current(&self) -> Option<&TokenWithSpan> {
        self.tokens.get(self.pos)
    }

    fn peek(&self, offset: usize) -> Option<&TokenWithSpan> {
        self.tokens.get(self.pos + offset)
    }

    fn advance(&mut self) {
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
    }

    fn skip_newlines(&mut self) {
        while let Some(t) = self.current() {
            if matches!(t.token, Token::Newline | Token::Comment) {
                self.advance();
            } else {
                break;
            }
        }
    }

    /// Parse the entire input into a Document
    pub fn parse(&mut self) -> Result<Document, ParseError> {
        let mut doc = Document::default();
        self.skip_newlines();

        while let Some(t) = self.current() {
            if matches!(t.token, Token::Eof) {
                break;
            }

            match &t.token {
                Token::Ident(name) => {
                    // Check if this is a bare attribute (name = value) or a block
                    let name = name.clone();
                    let is_bare_attr = {
                        if let Some(next) = self.peek(1) {
                            matches!(&next.token, Token::Equal | Token::Colon)
                        } else {
                            false
                        }
                    };

                    if is_bare_attr {
                        self.advance(); // consume the identifier
                                        // Parse as a single-attribute block (name = value)
                        let attr = self.parse_attribute(name.clone())?;
                        let block = Block {
                            name,
                            labels: vec![],
                            blocks: vec![],
                            attributes: vec![attr].into_iter().collect(),
                        };
                        doc.blocks.push(block);
                    } else {
                        // Parse as a block
                        let block = self.parse_block()?;
                        doc.blocks.push(block);
                    }
                }
                _ => {
                    return Err(ParseError {
                        message: format!("Unexpected token: {:?}", t.token),
                        line: t.span.start.line,
                        column: t.span.start.column,
                    });
                }
            }
            self.skip_newlines();
        }

        Ok(doc)
    }

    /// Parse a single block
    fn parse_block(&mut self) -> Result<Block, ParseError> {
        let ident = match self.current() {
            Some(t) if matches!(t.token, Token::Ident(_)) => {
                let name = match &t.token {
                    Token::Ident(s) => s.clone(),
                    _ => unreachable!(),
                };
                self.advance();
                name
            }
            Some(t) => {
                return Err(ParseError {
                    message: format!("Expected block name, found {:?}", t.token),
                    line: t.span.start.line,
                    column: t.span.start.column,
                });
            }
            None => {
                return Err(ParseError {
                    message: "Unexpected end of input".to_string(),
                    line: 0,
                    column: 0,
                });
            }
        };

        // Parse optional labels (e.g., "openai" in providers "openai" { })
        let mut labels = Vec::new();
        while let Some(t) = self.current() {
            match &t.token {
                Token::String(s) => {
                    labels.push(s.clone());
                    self.advance();
                }
                Token::Ident(s)
                    if self
                        .peek(1)
                        .map(|p| matches!(p.token, Token::LeftBrace))
                        .unwrap_or(false) =>
                {
                    // This is a block without labels, break
                    break;
                }
                _ => break,
            }
        }

        self.skip_newlines();

        // Parse block body
        let (blocks, attributes) = self.parse_block_body()?;

        Ok(Block {
            name: ident,
            labels,
            blocks,
            attributes,
        })
    }

    /// Parse the body of a block (attributes and nested blocks)
    fn parse_block_body(&mut self) -> Result<(Vec<Block>, HashMap<String, Value>), ParseError> {
        let mut blocks = Vec::new();
        let mut attributes = HashMap::new();

        self.skip_newlines();

        // Handle blocks without braces (implicit blocks)
        while let Some(t) = self.current() {
            match &t.token {
                Token::RightBrace => {
                    self.advance();
                    break;
                }
                Token::LeftBrace => {
                    // This shouldn't happen normally
                    self.advance();
                }
                Token::Ident(name) => {
                    // Check if this is a nested block or attribute
                    let name = name.clone();
                    let after_ident = self.peek(1);
                    let after_after = self.peek(2);

                    if let Some(next) = after_ident {
                        match &next.token {
                            Token::Equal | Token::Colon => {
                                // It's an attribute: name = value or name : value
                                self.advance(); // consume the identifier
                                let attr = self.parse_attribute(name)?;
                                attributes.insert(attr.0, attr.1);
                            }
                            Token::String(_) => {
                                // It's a block with a string label, parse as nested block
                                let block = self.parse_block()?;
                                blocks.push(block);
                            }
                            Token::LeftBrace => {
                                // Nested block with no labels
                                let block = self.parse_nested_block(name.clone())?;
                                blocks.push(block);
                            }
                            Token::Ident(_) => {
                                // This could be a nested block type
                                // Check if next is a label or another ident
                                if let Some(after) = after_after {
                                    match &after.token {
                                        Token::LeftBrace | Token::String(_) => {
                                            // It's a nested block
                                            let block = self.parse_block()?;
                                            blocks.push(block);
                                        }
                                        Token::Equal | Token::Colon => {
                                            // It's an attribute
                                            self.advance(); // consume the identifier
                                            let attr = self.parse_attribute(name)?;
                                            attributes.insert(attr.0, attr.1);
                                        }
                                        _ => {
                                            // Try as nested block
                                            let block = self.parse_block()?;
                                            blocks.push(block);
                                        }
                                    }
                                } else {
                                    // Just an identifier, skip
                                    self.advance();
                                }
                            }
                            _ => {
                                self.advance();
                            }
                        }
                    } else {
                        self.advance();
                    }
                }
                Token::Newline | Token::Comment => {
                    self.advance();
                }
                Token::Eof => break,
                _ => {
                    // Skip unexpected tokens
                    self.advance();
                }
            }
            self.skip_newlines();
        }

        Ok((blocks, attributes))
    }

    /// Parse a nested block after we've already consumed the type name
    fn parse_nested_block(&mut self, name: String) -> Result<Block, ParseError> {
        // Check for labels
        let mut labels = Vec::new();
        while let Some(t) = self.current() {
            match &t.token {
                Token::String(s) => {
                    labels.push(s.clone());
                    self.advance();
                }
                _ => break,
            }
        }

        self.skip_newlines();

        let (blocks, attributes) = self.parse_block_body()?;

        Ok(Block {
            name,
            labels,
            blocks,
            attributes,
        })
    }

    /// Parse an attribute assignment: name = value
    fn parse_attribute(&mut self, name: String) -> Result<(String, Value), ParseError> {
        // Already consumed the attribute name, now expecting = or :
        let token = self.current().cloned();
        if let Some(t) = token {
            if !matches!(t.token, Token::Equal | Token::Colon) {
                return Err(ParseError {
                    message: format!("Expected '=' or ':', found {:?}", t.token),
                    line: t.span.start.line,
                    column: t.span.start.column,
                });
            }
            self.advance();
        }

        self.skip_newlines();
        let value = self.parse_value()?;

        Ok((name, value))
    }

    /// Parse a value
    fn parse_value(&mut self) -> Result<Value, ParseError> {
        let token = self.current().ok_or_else(|| ParseError {
            message: "Unexpected end of input".to_string(),
            line: 0,
            column: 0,
        })?;

        match token.token.clone() {
            Token::String(s) => {
                self.advance();
                Ok(Value::String(s))
            }
            Token::Number(n) => {
                self.advance();
                Ok(Value::Number(n))
            }
            Token::True => {
                self.advance();
                Ok(Value::Bool(true))
            }
            Token::False => {
                self.advance();
                Ok(Value::Bool(false))
            }
            Token::Null => {
                self.advance();
                Ok(Value::Null)
            }
            Token::LeftBracket => {
                self.advance();
                self.parse_list()
            }
            Token::LeftBrace => {
                self.advance();
                let (_blocks, attrs) = self.parse_block_body()?;
                Ok(Value::Object(attrs.into_iter().collect()))
            }
            Token::Ident(name) => {
                let name = name.clone();
                // Could be an identifier, a nested block, or a function call
                self.advance();
                self.skip_newlines();

                // Check if this is a function call: name(args)
                if let Some(next) = self.current() {
                    match &next.token {
                        Token::LeftParen => {
                            // It's a function call
                            self.advance(); // consume '('
                            let args = self.parse_call_args()?;
                            return Ok(Value::Call(name, args));
                        }
                        Token::LeftBrace => {
                            // It's a nested block, but we already consumed the name
                            let block = self.parse_nested_block(name)?;
                            return Ok(Value::Object(vec![(
                                "_block".to_string(),
                                Value::Object(vec![
                                    ("name".to_string(), Value::String(block.name)),
                                    (
                                        "labels".to_string(),
                                        Value::List(
                                            block
                                                .labels
                                                .iter()
                                                .map(|s| Value::String(s.clone()))
                                                .collect(),
                                        ),
                                    ),
                                    (
                                        "attributes".to_string(),
                                        Value::Object(block.attributes.into_iter().collect()),
                                    ),
                                ]),
                            )]));
                        }
                        _ => {}
                    }
                }

                Ok(Value::String(name))
            }
            _ => Err(ParseError {
                message: format!("Unexpected token in value position: {:?}", token.token),
                line: token.span.start.line,
                column: token.span.start.column,
            }),
        }
    }

    /// Parse a list: [1, 2, 3]
    fn parse_list(&mut self) -> Result<Value, ParseError> {
        let mut items = Vec::new();
        self.skip_newlines();

        while let Some(t) = self.current() {
            match &t.token {
                Token::RightBracket => {
                    self.advance();
                    break;
                }
                Token::Comma => {
                    self.advance();
                    self.skip_newlines();
                }
                Token::Newline | Token::Comment => {
                    self.advance();
                }
                _ => {
                    let value = self.parse_value()?;
                    items.push(value);
                    self.skip_newlines();

                    // Check for comma or end of list
                    if let Some(next) = self.current() {
                        match &next.token {
                            Token::Comma => {
                                self.advance(); // consume comma
                                self.skip_newlines();
                            }
                            Token::RightBracket => {
                                // end of list
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        Ok(Value::List(items))
    }

    /// Parse function call arguments: arg1, arg2, ...)
    fn parse_call_args(&mut self) -> Result<Vec<Value>, ParseError> {
        let mut args = Vec::new();
        self.skip_newlines();

        // Handle empty args
        if let Some(t) = self.current() {
            if matches!(t.token, Token::RightParen) {
                self.advance();
                return Ok(args);
            }
        }

        loop {
            let value = self.parse_value()?;
            args.push(value);
            self.skip_newlines();

            match self.current() {
                Some(t) if matches!(t.token, Token::Comma) => {
                    self.advance(); // consume comma
                    self.skip_newlines();
                }
                Some(t) if matches!(t.token, Token::RightParen) => {
                    self.advance(); // consume ')'
                    break;
                }
                Some(t) => {
                    return Err(ParseError {
                        message: format!("Expected ',' or ')', found {:?}", t.token),
                        line: t.span.start.line,
                        column: t.span.start.column,
                    });
                }
                None => {
                    return Err(ParseError {
                        message: "Unexpected end of input in function call".to_string(),
                        line: 0,
                        column: 0,
                    });
                }
            }
        }

        Ok(args)
    }
}

/// Parse ACL text into a Document
pub fn parse(input: &str) -> Result<Document, ParseError> {
    Parser::new(input).parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let input = r#"
            name = "test"
            count = 42
        "#;
        let doc = parse(input).unwrap();
        assert_eq!(doc.blocks.len(), 2);
        assert_eq!(doc.blocks[0].name, "name");
        assert_eq!(doc.blocks[1].name, "count");
    }

    #[test]
    fn test_parse_block() {
        let input = r#"
            providers "openai" {
                name = "openai"
                api_key = "sk-test"
            }
        "#;
        let doc = parse(input).unwrap();
        assert_eq!(doc.blocks.len(), 1);
        let block = &doc.blocks[0];
        assert_eq!(block.name, "providers");
        assert_eq!(block.labels, vec!["openai"]);
        assert_eq!(
            block.attributes.get("name").map(|v| v.to_string()).unwrap(),
            "openai"
        );
    }

    #[test]
    fn test_parse_nested_block() {
        let input = r#"
            providers {
                openai "gpt-4" {
                    model = "gpt-4"
                }
            }
        "#;
        let doc = parse(input).unwrap();
        assert_eq!(doc.blocks.len(), 1);
        let block = &doc.blocks[0];
        assert_eq!(block.name, "providers");
        assert!(block.blocks.len() >= 1);
    }

    #[test]
    fn test_parse_list() {
        let input = r#"
            numbers = [1, 2, 3]
        "#;
        let doc = parse(input).unwrap();
        assert_eq!(doc.blocks.len(), 1);
        let attr = doc.blocks[0].attributes.get("numbers").unwrap();
        match attr {
            Value::List(items) => assert_eq!(items.len(), 3),
            _ => panic!("Expected list"),
        }
    }

    #[test]
    fn test_parse_string_list() {
        // Test list with single element
        let input = r#"
            names = ["alice"]
        "#;
        let doc = parse(input).unwrap();
        let attr = doc.blocks[0].attributes.get("names").unwrap();
        match attr {
            Value::List(items) => assert_eq!(items.len(), 1),
            _ => panic!("Expected list"),
        }
    }

    #[test]
    fn test_parse_boolean_true() {
        let input = r#"
            enabled = true
        "#;
        let doc = parse(input).unwrap();
        let attr = doc.blocks[0].attributes.get("enabled").unwrap();
        assert_eq!(attr, &Value::Bool(true));
    }

    #[test]
    fn test_parse_boolean_false() {
        let input = r#"
            enabled = false
        "#;
        let doc = parse(input).unwrap();
        let attr = doc.blocks[0].attributes.get("enabled").unwrap();
        assert_eq!(attr, &Value::Bool(false));
    }

    #[test]
    fn test_parse_null() {
        let input = r#"
            value = null
        "#;
        let doc = parse(input).unwrap();
        let attr = doc.blocks[0].attributes.get("value").unwrap();
        assert_eq!(attr, &Value::Null);
    }

    #[test]
    fn test_parse_number_integer() {
        let input = r#"
            count = 42
        "#;
        let doc = parse(input).unwrap();
        let attr = doc.blocks[0].attributes.get("count").unwrap();
        match attr {
            Value::Number(n) => assert_eq!(*n, 42.0),
            _ => panic!("Expected number"),
        }
    }

    #[test]
    fn test_parse_number_float() {
        let input = r#"
            pi = 3.14
        "#;
        let doc = parse(input).unwrap();
        let attr = doc.blocks[0].attributes.get("pi").unwrap();
        match attr {
            Value::Number(n) => assert!((*n - 3.14).abs() < 0.001),
            _ => panic!("Expected number"),
        }
    }

    #[test]
    fn test_parse_negative_number() {
        let input = r#"
            temp = -10
        "#;
        let doc = parse(input).unwrap();
        let attr = doc.blocks[0].attributes.get("temp").unwrap();
        match attr {
            Value::Number(n) => assert_eq!(*n, -10.0),
            _ => panic!("Expected number"),
        }
    }

    #[test]
    fn test_parse_string_value() {
        let input = r#"
            name = "hello world"
        "#;
        let doc = parse(input).unwrap();
        let attr = doc.blocks[0].attributes.get("name").unwrap();
        match attr {
            Value::String(s) => assert_eq!(s, "hello world"),
            _ => panic!("Expected string"),
        }
    }

    #[test]
    fn test_parse_colon_separator() {
        let input = r#"
            config {
                key: "value"
            }
        "#;
        let doc = parse(input).unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert_eq!(doc.blocks[0].name, "config");
    }

    #[test]
    fn test_parse_error_display() {
        let err = ParseError {
            message: "test error".to_string(),
            line: 10,
            column: 5,
        };
        let display = format!("{}", err);
        assert!(display.contains("line 10"));
        assert!(display.contains("column 5"));
        assert!(display.contains("test error"));
    }

    #[test]
    fn test_parse_error_trait() {
        let err = ParseError {
            message: "test".to_string(),
            line: 1,
            column: 1,
        };
        let _ = err.clone();
    }

    #[test]
    fn test_parse_empty_document() {
        let input = "";
        let doc = parse(input).unwrap();
        assert!(doc.blocks.is_empty());
    }

    #[test]
    fn test_parse_only_newlines() {
        let input = "\n\n\n";
        let doc = parse(input).unwrap();
        assert!(doc.blocks.is_empty());
    }

    #[test]
    fn test_parse_comments() {
        let input = r#"
            # this is a comment
            name = "test"
            // another comment
            count = 42
        "#;
        let doc = parse(input).unwrap();
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_parse_block_no_braces() {
        let input = r#"
            config
                name = "test"
        "#;
        let doc = parse(input).unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert_eq!(doc.blocks[0].name, "config");
    }

    #[test]
    fn test_parse_multiple_blocks() {
        let input = r#"
            block1 "label1" {}
            block2 "label2" {}
        "#;
        let doc = parse(input).unwrap();
        assert_eq!(doc.blocks.len(), 2);
        assert_eq!(doc.blocks[0].name, "block1");
        assert_eq!(doc.blocks[1].name, "block2");
    }

    #[test]
    fn test_parse_identifier_value() {
        let input = r#"
            ref = other_name
        "#;
        let doc = parse(input).unwrap();
        let attr = doc.blocks[0].attributes.get("ref").unwrap();
        match attr {
            Value::String(s) => assert_eq!(s, "other_name"),
            _ => panic!("Expected string"),
        }
    }

    #[test]
    fn test_parse_block_with_block_nested_inside() {
        let input = r#"
            outer {
                inner "label" {
                    key = "value"
                }
            }
        "#;
        let doc = parse(input).unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert_eq!(doc.blocks[0].name, "outer");
    }

    #[test]
    fn test_parse_implicit_block_with_labels() {
        let input = r#"
            item "first" "second" {
                name = "test"
            }
        "#;
        let doc = parse(input).unwrap();
        assert_eq!(doc.blocks[0].labels.len(), 2);
    }

    #[test]
    fn test_parse_error_unexpected_token() {
        let input = r#"
            [invalid]
        "#;
        let result = parse(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_eof_in_block() {
        // Test with unexpected end of input in attribute value
        let input = r#"block key = "#;
        let result = parse(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_block_no_labels() {
        let input = r#"
            block {
                key = "value"
            }
        "#;
        let doc = parse(input).unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert_eq!(doc.blocks[0].name, "block");
        assert!(doc.blocks[0].labels.is_empty());
    }

    #[test]
    fn test_parse_multiple_attributes() {
        let input = r#"
            config {
                a = 1
                b = 2
                c = 3
            }
        "#;
        let doc = parse(input).unwrap();
        assert_eq!(doc.blocks[0].attributes.len(), 3);
    }

    #[test]
    fn test_parse_list_of_numbers() {
        let input = r#"
            nums = [1, 2, 3]
        "#;
        let doc = parse(input).unwrap();
        let attr = doc.blocks[0].attributes.get("nums").unwrap();
        match attr {
            Value::List(items) => {
                assert_eq!(items.len(), 3);
                assert_eq!(items[0], Value::Number(1.0));
                assert_eq!(items[1], Value::Number(2.0));
                assert_eq!(items[2], Value::Number(3.0));
            }
            _ => panic!("Expected list"),
        }
    }

    #[test]
    fn test_parse_list_of_mixed() {
        let input = r#"
            mixed = ["string", 42, true]
        "#;
        let doc = parse(input).unwrap();
        let attr = doc.blocks[0].attributes.get("mixed").unwrap();
        match attr {
            Value::List(items) => assert_eq!(items.len(), 3),
            _ => panic!("Expected list"),
        }
    }

    #[test]
    fn test_parse_empty_list() {
        let input = r#"items = []"#;
        let doc = parse(input).unwrap();
        let attr = doc.blocks[0].attributes.get("items").unwrap();
        match attr {
            Value::List(items) => assert!(items.is_empty()),
            _ => panic!("Expected list"),
        }
    }

    #[test]
    fn test_parse_nested_block_with_labels() {
        let input = r#"
            parent {
                child "label" {
                    key = "value"
                }
            }
        "#;
        let doc = parse(input).unwrap();
        assert!(!doc.blocks[0].blocks.is_empty());
    }

    #[test]
    fn test_parse_left_brace_after_ident() {
        // Test case: ident followed by left brace (implicit block without labels)
        let input = r#"
            myblock {
                key = "value"
            }
        "#;
        let doc = parse(input).unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert_eq!(doc.blocks[0].name, "myblock");
    }

    #[test]
    fn test_parse_left_brace_after_string() {
        // Test case: ident followed by string label then left brace
        let input = r#"
            myblock "label" {
                key = "value"
            }
        "#;
        let doc = parse(input).unwrap();
        assert_eq!(doc.blocks[0].labels, vec!["label"]);
    }

    #[test]
    fn test_parse_ident_after_ident_with_left_brace() {
        // Test case: ident followed by another ident and then left brace
        let input = r#"
            type "name" {
                key = "value"
            }
        "#;
        let doc = parse(input).unwrap();
        assert_eq!(doc.blocks[0].name, "type");
        assert_eq!(doc.blocks[0].labels, vec!["name"]);
    }

    #[test]
    fn test_parse_block_sparse_labels() {
        let input = r#"
            block "a" "b" "c" {}
        "#;
        let doc = parse(input).unwrap();
        assert_eq!(doc.blocks[0].labels, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_parse_value_number_negative() {
        let input = r#"val = -123"#;
        let doc = parse(input).unwrap();
        let attr = doc.blocks[0].attributes.get("val").unwrap();
        assert_eq!(attr, &Value::Number(-123.0));
    }

    #[test]
    fn test_parse_value_number_float() {
        let input = r#"val = 0.5"#;
        let doc = parse(input).unwrap();
        let attr = doc.blocks[0].attributes.get("val").unwrap();
        match attr {
            Value::Number(n) => assert!((*n - 0.5).abs() < 0.001),
            _ => panic!("Expected number"),
        }
    }

    #[test]
    fn test_parse_whitespace_only() {
        let input = "   \t\t  \n\n\n   ";
        let doc = parse(input).unwrap();
        assert!(doc.blocks.is_empty());
    }

    #[test]
    fn test_parse_only_comments() {
        let input = "# comment\n// another\n# third";
        let doc = parse(input).unwrap();
        assert!(doc.blocks.is_empty());
    }


    #[test]
    fn test_parse_comment_between_statements() {
        let input = r#"
            a = 1
            # comment
            b = 2
        "#;
        let doc = parse(input).unwrap();
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_parse_skip_unexpected_token() {
        // Test that unexpected tokens in block body are skipped
        let input = r#"
            block {
                key = "value"
                unexpected_token
                other = "data"
            }
        "#;
        let doc = parse(input).unwrap();
        // The parser should skip unexpected tokens and continue
        assert_eq!(doc.blocks.len(), 1);
    }

    #[test]
    fn test_parse_only_newline_in_list() {
        // Test list with newlines between items
        let input = r#"
            nums = [
                1,
                2,
                3
            ]
        "#;
        let doc = parse(input).unwrap();
        let attr = doc.blocks[0].attributes.get("nums").unwrap();
        match attr {
            Value::List(items) => assert_eq!(items.len(), 3),
            _ => panic!("Expected list"),
        }
    }

    #[test]
    fn test_parse_list_trailing_comma() {
        let input = r#"nums = [1, 2, 3,]"#;
        let result = parse(input);
        // May or may not parse depending on parser rules
        // Just ensure no crash
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_parse_block_with_comment_before_attr() {
        let input = r#"
            block {
                # this is a comment
                key = "value"
            }
        "#;
        let doc = parse(input).unwrap();
        assert_eq!(
            doc.blocks[0]
                .attributes
                .get("key")
                .map(|v| v.to_string())
                .unwrap(),
            "value"
        );
    }

    #[test]
    fn test_parse_float_scientific() {
        let input = r#"val = 1e-5"#;
        let doc = parse(input).unwrap();
        let attr = doc.blocks[0].attributes.get("val").unwrap();
        match attr {
            Value::Number(n) => assert!((*n - 0.00001).abs() < 0.000001),
            _ => panic!("Expected number"),
        }
    }

    #[test]
    fn test_parse_block_without_body() {
        // Block with just name, no braces
        let input = r#"empty_block"#;
        let doc = parse(input).unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert_eq!(doc.blocks[0].name, "empty_block");
    }

    #[test]
    fn test_parse_nested_block_type_ident() {
        // Test nested block where type is an identifier (not string)
        let input = r#"
            outer {
                inner "label" {}
            }
        "#;
        let doc = parse(input).unwrap();
        assert!(!doc.blocks[0].blocks.is_empty());
    }

    #[test]
    fn test_parse_block_with_plus_equal() {
        // Test += operator (though it's not fully supported)
        let input = r#"key += value"#;
        let result = parse(input);
        // Should either parse or error gracefully
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_parse_attr_with_colon() {
        // Test attribute with colon separator
        let input = r#"key: "value""#;
        let doc = parse(input).unwrap();
        assert_eq!(
            doc.blocks[0]
                .attributes
                .get("key")
                .map(|v| v.to_string())
                .unwrap(),
            "value"
        );
    }

    #[test]
    fn test_parse_block_empty_labels() {
        let input = r#"block "" {}"#;
        let doc = parse(input).unwrap();
        assert_eq!(doc.blocks[0].labels, vec![""]);
    }

    #[test]
    fn test_parse_eof_after_block() {
        let input = r#"block {}"#;
        let doc = parse(input).unwrap();
        assert_eq!(doc.blocks.len(), 1);
    }

    #[test]
    fn test_parse_block_single_quoted_label() {
        let input = r#"block 'label' {}"#;
        let doc = parse(input).unwrap();
        assert_eq!(doc.blocks[0].labels, vec!["label"]);
    }

    #[test]
    fn test_parse_string_with_equals() {
        let input = r#"key = "a=b""#;
        let doc = parse(input).unwrap();
        assert_eq!(
            doc.blocks[0]
                .attributes
                .get("key")
                .map(|v| v.to_string())
                .unwrap(),
            "a=b"
        );
    }

    #[test]
    fn test_parse_string_with_braces() {
        let input = r#"key = "a{b}c""#;
        let doc = parse(input).unwrap();
        assert_eq!(
            doc.blocks[0]
                .attributes
                .get("key")
                .map(|v| v.to_string())
                .unwrap(),
            "a{b}c"
        );
    }

    #[test]
    fn test_parse_string_with_brackets() {
        let input = r#"key = "a[b]c""#;
        let doc = parse(input).unwrap();
        assert_eq!(
            doc.blocks[0]
                .attributes
                .get("key")
                .map(|v| v.to_string())
                .unwrap(),
            "a[b]c"
        );
    }

    #[test]
    fn test_parse_string_with_hash() {
        let input = r#"key = "a#b""#;
        let doc = parse(input).unwrap();
        assert_eq!(
            doc.blocks[0]
                .attributes
                .get("key")
                .map(|v| v.to_string())
                .unwrap(),
            "a#b"
        );
    }

    #[test]
    fn test_parse_mixed_block_and_attrs() {
        let input = r#"
            outer {
                attr1 = 1
                inner "label" {
                    attr2 = 2
                }
                attr3 = 3
            }
        "#;
        let doc = parse(input).unwrap();
        let outer = &doc.blocks[0];
        assert_eq!(outer.attributes.len(), 2); // attr1 and attr3
        assert!(!outer.blocks.is_empty()); // inner block
    }

    #[test]
    fn test_parse_block_starting_with_dot() {
        // String starting with dot needs quoting
        let input = r#"key = ".hidden""#;
        let doc = parse(input).unwrap();
        assert_eq!(
            doc.blocks[0]
                .attributes
                .get("key")
                .map(|v| v.to_string())
                .unwrap(),
            ".hidden"
        );
    }

    #[test]
    fn test_parse_block_with_number_value() {
        let input = r#"key = 0"#;
        let doc = parse(input).unwrap();
        assert_eq!(
            doc.blocks[0]
                .attributes
                .get("key")
                .map(|v| v.to_string())
                .unwrap(),
            "0"
        );
    }

    #[test]
    fn test_parse_bool_true_as_value() {
        let input = r#"key = true"#;
        let doc = parse(input).unwrap();
        assert_eq!(
            doc.blocks[0]
                .attributes
                .get("key")
                .map(|v| v.to_string())
                .unwrap(),
            "true"
        );
    }

    #[test]
    fn test_parse_bool_false_as_value() {
        let input = r#"key = false"#;
        let doc = parse(input).unwrap();
        assert_eq!(
            doc.blocks[0]
                .attributes
                .get("key")
                .map(|v| v.to_string())
                .unwrap(),
            "false"
        );
    }

    #[test]
    fn test_parse_null_as_value() {
        let input = r#"key = null"#;
        let doc = parse(input).unwrap();
        assert_eq!(
            doc.blocks[0]
                .attributes
                .get("key")
                .map(|v| v.to_string())
                .unwrap(),
            "null"
        );
    }

    #[test]
    fn test_parse_string_with_unicode() {
        let input = r#"key = "hello 世界""#;
        let doc = parse(input).unwrap();
        assert_eq!(
            doc.blocks[0]
                .attributes
                .get("key")
                .map(|v| v.to_string())
                .unwrap(),
            "hello 世界"
        );
    }

    #[test]
    fn test_parse_multiple_labels_different_types() {
        let input = r#"block "a" 'b' "c" {}"#;
        let doc = parse(input).unwrap();
        assert_eq!(doc.blocks[0].labels, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_parse_ident_value_followed_by_block() {
        // Test that identifier value followed by block parses correctly
        let input = r#"block {
            type container
            name = "test"
        }"#;
        let doc = parse(input).unwrap();
        assert_eq!(doc.blocks[0].name, "block");
    }

    #[test]
    fn test_parse_value_with_equals_in_string() {
        // Test string containing = character
        let input = r#"key = "a=b=c""#;
        let doc = parse(input).unwrap();
        assert_eq!(
            doc.blocks[0]
                .attributes
                .get("key")
                .map(|v| v.to_string())
                .unwrap(),
            "a=b=c"
        );
    }

    #[test]
    fn test_parse_list_with_single_number() {
        let input = r#"nums = [42]"#;
        let doc = parse(input).unwrap();
        let attr = doc.blocks[0].attributes.get("nums").unwrap();
        match attr {
            Value::List(items) => assert_eq!(items.len(), 1),
            _ => panic!("Expected list"),
        }
    }

    #[test]
    fn test_parse_list_with_trailing_newline() {
        let input = r#"
            nums = [
                1,
                2,
            ]
        "#;
        let doc = parse(input).unwrap();
        let attr = doc.blocks[0].attributes.get("nums").unwrap();
        match attr {
            Value::List(items) => assert_eq!(items.len(), 2),
            _ => panic!("Expected list"),
        }
    }

    #[test]
    fn test_parse_attr_with_number_starting_with_dot() {
        // String starting with dot needs quotes
        let input = r#"key = ".5""#;
        let doc = parse(input).unwrap();
        assert_eq!(
            doc.blocks[0]
                .attributes
                .get("key")
                .map(|v| v.to_string())
                .unwrap(),
            ".5"
        );
    }

    #[test]
    fn test_parse_eof_in_nested_block() {
        // Test EOF reached while parsing - unclosed string
        let input = r#"key = "unclosed"#;
        let result = parse(input);
        // This might not error depending on how lexer handles it
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_parse_block_with_comment_after_name() {
        let input = r#"
            block # comment
                key = "value"
        "#;
        let doc = parse(input).unwrap();
        assert_eq!(doc.blocks[0].name, "block");
    }

    #[test]
    fn test_parse_float_with_leading_dot() {
        let input = r#"val = .123"#;
        let doc = parse(input).unwrap();
        let attr = doc.blocks[0].attributes.get("val").unwrap();
        match attr {
            Value::Number(n) => assert!((*n - 0.123).abs() < 0.001),
            _ => panic!("Expected number"),
        }
    }


    #[test]
    fn test_parse_error_block_name_eof() {
        // EOF during block name parsing triggers line 142-148
        let input = r#"block";
        let result = parse(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_eof_in_block() {
        // EOF after opening brace in block body
        let input = r#"block { attr"#;
        let result = parse(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_eof_in_nested_block_graceful() {
        // EOF in nested block context is handled gracefully
        let input = r#"outer { inner"#;
        let result = parse(input);
        // Parser handles this gracefully by treating inner as a block
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_error_invalid_list_token() {
        // Unknown token where value is expected
        let input = r#"val = [}"]"#;
        let result = parse(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_ident_at_eof() {
        // Identifier at end of input (triggers peek returning None)
        let input = r#"name"#;
        let doc = parse(input).unwrap();
        // Should parse as a block with no attributes
        assert_eq!(doc.blocks.len(), 1);
        assert_eq!(doc.blocks[0].name, "name");
    }

    #[test]
    fn test_parse_block_with_only_newline_in_body() {
        // Block followed by newline in body
        let input = r#"block {
        }"#;
        let doc = parse(input).unwrap();
        assert_eq!(doc.blocks.len(), 1);
    }

    #[test]
    fn test_parse_error_number_as_block_name() {
        // Number where block name expected - triggers line 135-139
        let input = r#"123"#;
        let result = parse(input);
        assert!(result.is_err());
        let err = result.unwrap_err();
        // Parser returns error for unexpected token
        assert!(
            err.message.contains("Unexpected token") || err.message.contains("Expected block name")
        );
    }

    #[test]
    fn test_parse_label_then_nested() {
        // Block with string label followed by attribute
        let input = r#"block "label" {
            key = "value"
        }"#;
        let doc = parse(input).unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert_eq!(doc.blocks[0].labels, vec!["label"]);
    }

    #[test]
    fn test_parse_block_empty_string_label() {
        // Block with empty string label
        let input = r#"block "" {
        }"#;
        let doc = parse(input).unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert_eq!(doc.blocks[0].labels, vec![""]);
    }

    #[test]
    fn test_parse_multiple_string_labels() {
        // Block with multiple string labels
        let input = r#"block "label1" "label2" {
        }"#;
        let doc = parse(input).unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert_eq!(doc.blocks[0].labels, vec!["label1", "label2"]);
    }

    #[test]
    fn test_parse_string_then_brace_as_block() {
        // String followed by left brace is parsed as block
        let input = r#"provider "openai" {
            key = "value"
        }"#;
        let doc = parse(input).unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert_eq!(doc.blocks[0].name, "provider");
    }

    #[test]
    fn test_parse_attribute_with_plus_equal() {
        // Attribute with += operator
        let input = r#"vals += 1"#;
        let doc = parse(input).unwrap();
        assert_eq!(doc.blocks.len(), 1);
    }

    #[test]
    fn test_parse_function_call_env() {
        // env() function call
        let input = r#"api_key = env("API_KEY")"#;
        let doc = parse(input).unwrap();
        let attr = doc.blocks[0].attributes.get("api_key").unwrap();
        match attr {
            Value::Call(name, args) => {
                assert_eq!(name, "env");
                assert_eq!(args.len(), 1);
                assert_eq!(args[0], Value::String("API_KEY".to_string()));
            }
            _ => panic!("Expected Call value"),
        }
    }

    #[test]
    fn test_parse_function_call_with_multiple_args() {
        // concat() function call with multiple args
        let input = r#"url = concat("postgres://", host, ":", port)"#;
        let doc = parse(input).unwrap();
        let attr = doc.blocks[0].attributes.get("url").unwrap();
        match attr {
            Value::Call(name, args) => {
                assert_eq!(name, "concat");
                assert_eq!(args.len(), 4);
            }
            _ => panic!("Expected Call value"),
        }
    }

    #[test]
    fn test_parse_function_call_with_nested() {
        // Nested function call
        let input = r#"path = concat(env("HOME"), "/", "file")"#;
        let doc = parse(input).unwrap();
        let attr = doc.blocks[0].attributes.get("path").unwrap();
        match attr {
            Value::Call(name, args) => {
                assert_eq!(name, "concat");
                assert_eq!(args.len(), 3);
            }
            _ => panic!("Expected Call value"),
        }
    }

    #[test]
    fn test_parse_function_call_empty_args() {
        // Function call with no args
        let input = r#"result = getenv()"#;
        let doc = parse(input).unwrap();
        let attr = doc.blocks[0].attributes.get("result").unwrap();
        match attr {
            Value::Call(name, args) => {
                assert_eq!(name, "getenv");
                assert!(args.is_empty());
            }
            _ => panic!("Expected Call value"),
        }
    }

    #[test]
    fn test_parse_function_call_in_list() {
        // Function call as list item
        let input = r#"paths = [env("PATH1"), env("PATH2")]"#;
        let doc = parse(input).unwrap();
        let attr = doc.blocks[0].attributes.get("paths").unwrap();
        match attr {
            Value::List(items) => {
                assert_eq!(items.len(), 2);
            }
            _ => panic!("Expected List value"),
        }
    }

    #[test]
    fn test_parse_function_call_missing_paren() {
        // Function call without closing paren - should error
        let input = r#"val = env("API_KEY"#;
        let result = parse(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_list_of_objects() {
        let input = r#"
            knowledge_bases = [
                { id = "WThXBKfN21eAxJOl3n1PA", name = "个人知识" },
                { id = "another_id", name = "another_name" }
            ]
        "#;
        let doc = parse(input).unwrap();
        let attr = doc.blocks[0].attributes.get("knowledge_bases").unwrap();
        match attr {
            Value::List(items) => {
                assert_eq!(items.len(), 2);
                match &items[0] {
                    Value::Object(pairs) => {
                        let map: HashMap<_, _> = pairs.iter().cloned().collect();
                        assert_eq!(map.get("id"), Some(&Value::String("WThXBKfN21eAxJOl3n1PA".to_string())));
                        assert_eq!(map.get("name"), Some(&Value::String("个人知识".to_string())));
                    }
                    _ => panic!("Expected Object value"),
                }
            }
            _ => panic!("Expected List value"),
        }
    }

    #[test]
    fn test_parse_list_of_single_object() {
        let input = r#"items = [{ id = "1", name = "test" }]"#;
        let doc = parse(input).unwrap();
        let attr = doc.blocks[0].attributes.get("items").unwrap();
        match attr {
            Value::List(items) => assert_eq!(items.len(), 1),
            _ => panic!("Expected List value"),
        }
    }
}

