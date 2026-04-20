// ============================================================================
// ACL - Agent Configuration Language
//
// A configuration language similar to HCL for defining agent configurations.
// ============================================================================

pub mod ast;
pub mod generator;
pub mod lexer;
pub mod parser;

pub use ast::{Block, Document, Value};
pub use generator::{generate, generate_from_map, Generator, GeneratorConfig};
pub use lexer::{Lexer, Location, Span, Token, TokenWithSpan};
pub use parser::{parse, ParseError};

/// Parse ACL text into a Document
///
/// # Example
///
/// ```
/// use acl::parse;
///
/// let input = r#"
///     default_model = "openai/gpt-4"
///
///     providers "openai" {
///         name = "openai"
///         api_key = "sk-test"
///         base_url = "https://api.openai.com/v1"
///     }
/// "#;
///
/// let doc = parse(input).unwrap();
/// for block in doc.blocks {
///     println!("Block: {}", block.name);
/// }
/// ```
pub fn parse_acl(input: &str) -> Result<Document, ParseError> {
    parse(input)
}

/// Generate ACL text from a Document
///
/// # Example
///
/// ```
/// use acl::{generate, Document, Block, Value};
/// use std::collections::HashMap;
///
/// let mut attrs = HashMap::new();
/// attrs.insert("name".to_string(), Value::String("test".to_string()));
///
/// let doc = Document {
///     blocks: vec![Block {
///         name: "config".to_string(),
///         labels: vec![],
///         blocks: vec![],
///         attributes: attrs,
///     }],
/// };
///
/// let output = generate(&doc);
/// println!("{}", output);
/// ```
pub fn generate_acl(doc: &Document) -> String {
    generate(doc)
}

/// High-level builder API for creating ACL configurations
pub mod builder {
    use crate::ast::{Block, Document, Value};
    use std::collections::HashMap;

    /// Builder for ACL Documents
    pub struct DocumentBuilder {
        blocks: Vec<Block>,
    }

    impl DocumentBuilder {
        pub fn new() -> Self {
            Self { blocks: Vec::new() }
        }

        /// Add a block to the document
        pub fn block(mut self, block: Block) -> Self {
            self.blocks.push(block);
            self
        }

        /// Add a simple key-value block
        pub fn kv_block(mut self, name: &str, key: &str, value: Value) -> Self {
            let mut attrs = HashMap::new();
            attrs.insert(key.to_string(), value);
            self.blocks.push(Block {
                name: name.to_string(),
                labels: Vec::new(),
                blocks: Vec::new(),
                attributes: attrs,
            });
            self
        }

        /// Build the document
        pub fn build(self) -> Document {
            Document {
                blocks: self.blocks,
            }
        }
    }

    impl Default for DocumentBuilder {
        fn default() -> Self {
            Self::new()
        }
    }

    /// Builder for Blocks
    pub struct BlockBuilder {
        name: String,
        labels: Vec<String>,
        blocks: Vec<Block>,
        attributes: HashMap<String, Value>,
    }

    impl BlockBuilder {
        pub fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                labels: Vec::new(),
                blocks: Vec::new(),
                attributes: HashMap::new(),
            }
        }

        pub fn label(mut self, label: &str) -> Self {
            self.labels.push(label.to_string());
            self
        }

        pub fn attr(mut self, key: &str, value: Value) -> Self {
            self.attributes.insert(key.to_string(), value);
            self
        }

        pub fn nested_block(mut self, block: Block) -> Self {
            self.blocks.push(block);
            self
        }

        pub fn build(self) -> Block {
            Block {
                name: self.name,
                labels: self.labels,
                blocks: self.blocks,
                attributes: self.attributes,
            }
        }
    }

    // Helper functions for creating Values
    pub fn string(s: &str) -> Value {
        Value::String(s.to_string())
    }

    pub fn number(n: f64) -> Value {
        Value::Number(n)
    }

    pub fn integer(n: i64) -> Value {
        Value::Number(n as f64)
    }

    pub fn boolean(b: bool) -> Value {
        Value::Bool(b)
    }

    pub fn null() -> Value {
        Value::Null
    }

    pub fn list(items: Vec<Value>) -> Value {
        Value::List(items)
    }

    pub fn call(name: &str, args: Vec<Value>) -> Value {
        Value::Call(name.to_string(), args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_acl_function() {
        let input = r#"
            name = "test"
            count = 42
        "#;
        let doc = parse_acl(input).unwrap();
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_generate_acl_function() {
        let doc = Document {
            blocks: vec![Block {
                name: "test".to_string(),
                labels: vec![],
                blocks: vec![],
                attributes: vec![("key".to_string(), Value::String("value".to_string()))]
                    .into_iter()
                    .collect(),
            }],
        };
        let output = generate_acl(&doc);
        assert!(output.contains("test"));
        assert!(output.contains("value"));
    }

    #[test]
    fn test_builder_document_builder() {
        let doc = builder::DocumentBuilder::new()
            .kv_block("config", "key", builder::string("value"))
            .build();

        assert_eq!(doc.blocks.len(), 1);
        assert_eq!(doc.blocks[0].name, "config");
    }

    #[test]
    fn test_builder_document_builder_with_block() {
        let block = builder::BlockBuilder::new("provider")
            .attr("name", builder::string("openai"))
            .build();

        let doc = builder::DocumentBuilder::new().block(block).build();

        assert_eq!(doc.blocks.len(), 1);
        assert_eq!(doc.blocks[0].name, "provider");
    }

    #[test]
    fn test_builder_document_default() {
        let doc = builder::DocumentBuilder::default().build();
        assert!(doc.blocks.is_empty());
    }

    #[test]
    fn test_builder_block_builder() {
        let block = builder::BlockBuilder::new("provider")
            .label("openai")
            .attr("api_key", builder::string("sk-xxx"))
            .attr("enabled", builder::boolean(true))
            .build();

        assert_eq!(block.name, "provider");
        assert_eq!(block.labels, vec!["openai"]);
        assert_eq!(
            block
                .attributes
                .get("api_key")
                .map(|v| v.to_string())
                .unwrap(),
            "sk-xxx"
        );
    }

    #[test]
    fn test_builder_block_builder_nested() {
        let inner = builder::BlockBuilder::new("model")
            .attr("name", builder::string("gpt-4"))
            .build();

        let block = builder::BlockBuilder::new("provider")
            .label("openai")
            .nested_block(inner)
            .build();

        assert_eq!(block.blocks.len(), 1);
        assert_eq!(block.blocks[0].name, "model");
    }

    #[test]
    fn test_builder_value_helpers() {
        assert_eq!(builder::string("test"), Value::String("test".to_string()));
        assert_eq!(builder::number(3.14), Value::Number(3.14));
        assert_eq!(builder::integer(42), Value::Number(42.0));
        assert_eq!(builder::boolean(true), Value::Bool(true));
        assert_eq!(builder::boolean(false), Value::Bool(false));
        assert_eq!(builder::null(), Value::Null);
        assert_eq!(
            builder::list(vec![Value::Number(1.0)]),
            Value::List(vec![Value::Number(1.0)])
        );
    }

    #[test]
    fn test_re_exports() {
        // Verify re-exported types are accessible
        let _ = Document::default();
        let _ = Block {
            name: "test".to_string(),
            labels: vec![],
            blocks: vec![],
            attributes: std::collections::HashMap::new(),
        };
        let _ = Value::String("test".to_string());
        let _ = Generator::new();
        let _ = GeneratorConfig::default();
        let _ = Lexer::new("");
        let _ = Location {
            line: 1,
            column: 1,
            offset: 0,
        };
        let _ = Span::default();
        let _ = Token::Eof;
        let _ = TokenWithSpan::new(Token::Eof, Location::default(), Location::default());
        let _ = ParseError {
            message: "test".to_string(),
            line: 1,
            column: 1,
        };
    }

    #[test]
    fn test_lexer_re_exports() {
        let mut lexer = Lexer::new("test = 42");
        let tokens = lexer.tokenize();
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_token_with_span() {
        let loc = Location {
            line: 1,
            column: 1,
            offset: 0,
        };
        let token = TokenWithSpan::new(Token::Ident("test".to_string()), loc, loc);
        assert!(matches!(token.token, Token::Ident(_)));
    }
}
