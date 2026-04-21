// ============================================================================
// ACL Generator - Generate ACL text from structured data
// ============================================================================

use crate::ast::{Block, Document, Value};
use std::collections::HashMap;

/// Configuration for the generator
#[derive(Debug, Clone)]
pub struct GeneratorConfig {
    /// Indent string (default: "  " - two spaces)
    pub indent: &'static str,
    /// Enable inline comments
    pub comments: bool,
    /// Output labels as attributes (e.g., `name = "label"`) instead of block labels (e.g., `block "label" { }`)
    /// This generates HCL-compatible output instead of ACL-style labeled blocks
    pub labels_as_attrs: bool,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            indent: "  ",
            comments: false,
            labels_as_attrs: false,
        }
    }
}

/// ACL Generator
pub struct Generator {
    config: GeneratorConfig,
}

impl Generator {
    pub fn new() -> Self {
        Self {
            config: GeneratorConfig::default(),
        }
    }

    pub fn with_config(config: GeneratorConfig) -> Self {
        Self { config }
    }

    /// Generate ACL text from a Document
    pub fn generate(&self, doc: &Document) -> String {
        let mut output = String::new();
        self.write_document(doc, &mut output, 0);
        output
    }

    fn write_document(&self, doc: &Document, out: &mut String, indent: usize) {
        for (i, block) in doc.blocks.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            self.write_block(block, out, indent);
        }
    }

    fn write_block(&self, block: &Block, out: &mut String, indent: usize) {
        self.write_indent(out, indent);

        // Check if this is a single-value block (no nested blocks, only one attribute)
        if block.labels.is_empty() && block.blocks.is_empty() && block.attributes.len() == 1 {
            let (_key, value) = block.attributes.iter().next().unwrap();
            if value.is_string() && !value.as_str().unwrap().contains(' ') {
                out.push_str(&block.name);
                out.push_str(" = ");
                self.write_value(value, out);
                out.push('\n');
                return;
            }
        }

        // Write block header
        out.push_str(&block.name);

        // In labels_as_attrs mode, don't output labels in block header
        // Instead, we'll output them as attributes inside the block
        if !self.config.labels_as_attrs {
            for label in &block.labels {
                out.push(' ');
                self.write_string(label, out);
            }
        }

        if block.blocks.is_empty()
            && block.attributes.is_empty()
            && (self.config.labels_as_attrs || block.labels.is_empty())
        {
            out.push('\n');
            return;
        }

        out.push_str(" {\n");

        // In labels_as_attrs mode, output the first label as a "name" attribute
        if self.config.labels_as_attrs && !block.labels.is_empty() {
            self.write_indent(out, indent + 1);
            out.push_str("name = ");
            self.write_string(&block.labels[0], out);
            out.push('\n');
        }

        // Write attributes
        let mut attrs: Vec<_> = block.attributes.iter().collect();
        attrs.sort_by(|a, b| a.0.cmp(b.0));

        for (key, value) in attrs {
            self.write_indent(out, indent + 1);
            out.push_str(key);
            out.push_str(" = ");
            self.write_value(value, out);
            out.push('\n');
        }

        // Write nested blocks
        for block in &block.blocks {
            self.write_block(block, out, indent + 1);
            out.push('\n');
        }

        self.write_indent(out, indent);
        out.push('}');
    }

    fn write_value(&self, value: &Value, out: &mut String) {
        match value {
            Value::String(s) => self.write_string(s, out),
            Value::Number(n) => {
                if n.fract() == 0.0 {
                    out.push_str(&format!("{}", *n as i64));
                } else {
                    out.push_str(&format!("{}", n));
                }
            }
            Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Value::Null => out.push_str("null"),
            Value::List(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    self.write_value(item, out);
                }
                out.push(']');
            }
            Value::Object(pairs) => {
                out.push('{');
                for (i, (key, value)) in pairs.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    out.push_str(key);
                    out.push_str(" = ");
                    self.write_value(value, out);
                }
                out.push('}');
            }
            Value::Call(name, args) => {
                out.push_str(name);
                out.push('(');
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    self.write_value(arg, out);
                }
                out.push(')');
            }
        }
    }

    fn write_string(&self, s: &str, out: &mut String) {
        // Check if the string needs quotes
        let needs_quotes = s.is_empty()
            || s.contains(' ')
            || s.contains('#')
            || s.contains('"')
            || s.contains('\'')
            || s.contains('\\')
            || s.contains('\n')
            || s.contains('\r')
            || s.contains('\t')
            || s.contains(':')
            || s.contains('=')
            || s.contains('{')
            || s.contains('}')
            || s.contains('[')
            || s.contains(']')
            || s.starts_with('-')
            || s.starts_with('.')
            || s == "true"
            || s == "false"
            || s == "null"
            || s.chars()
                .next()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false);

        if needs_quotes {
            // Use double quotes with escaping
            out.push('"');
            for c in s.chars() {
                match c {
                    '"' => out.push_str("\\\""),
                    '\\' => out.push_str("\\\\"),
                    '\n' => out.push_str("\\n"),
                    '\r' => out.push_str("\\r"),
                    '\t' => out.push_str("\\t"),
                    _ => out.push(c),
                }
            }
            out.push('"');
        } else {
            out.push_str(s);
        }
    }

    fn write_indent(&self, out: &mut String, indent: usize) {
        for _ in 0..indent {
            out.push_str(self.config.indent);
        }
    }

    /// Generate ACL from a simple key-value structure
    pub fn generate_from_map(&self, data: &HashMap<String, Value>) -> String {
        let mut doc = Document::default();

        // Convert each top-level key to a block
        for (key, value) in data {
            let mut block = Block {
                name: key.clone(),
                labels: Vec::new(),
                blocks: Vec::new(),
                attributes: HashMap::new(),
            };

            if let Value::Object(pairs) = value {
                for (k, v) in pairs {
                    block.attributes.insert(k.clone(), v.clone());
                }
            } else {
                block.attributes.insert("_value".to_string(), value.clone());
            }

            doc.blocks.push(block);
        }

        self.generate(&doc)
    }
}

impl Default for Generator {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate ACL from a Document
pub fn generate(doc: &Document) -> String {
    Generator::new().generate(doc)
}

/// Generate ACL from a HashMap
pub fn generate_from_map(data: &HashMap<String, Value>) -> String {
    Generator::new().generate_from_map(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_simple() {
        let mut attrs = HashMap::new();
        attrs.insert("name".to_string(), Value::String("test".to_string()));

        let block = Block {
            name: "name".to_string(),
            labels: Vec::new(),
            blocks: Vec::new(),
            attributes: attrs,
        };

        let doc = Document {
            blocks: vec![block],
        };

        let output = generate(&doc);
        assert!(output.contains("name = test"));
    }

    #[test]
    fn test_string_escaping() {
        let gen = Generator::new();
        let mut out = String::new();
        gen.write_string("hello world", &mut out);
        assert_eq!(out, "\"hello world\"");

        out.clear();
        gen.write_string("hello\"world", &mut out);
        assert_eq!(out, "\"hello\\\"world\"");

        // Test string with tab character (actual tab, not escaped)
        out.clear();
        let with_tab = "line\tbreak";
        gen.write_string(with_tab, &mut out);
        // Tab needs quotes and escaping
        assert!(out.contains("line"));
        assert!(out.contains("break"));

        // Test backslash escape (actual backslash character)
        out.clear();
        let with_backslash = "path\\file";
        gen.write_string(with_backslash, &mut out);
        assert!(out.contains("path"));
        assert!(out.contains("file"));
    }

    #[test]
    fn test_string_escape_carriage_return() {
        let gen = Generator::new();
        let mut out = String::new();
        // Create a string with actual carriage return character
        let with_cr = String::from("line") + &"\r" + "break";
        assert!(with_cr.contains('\r')); // Verify it contains CR

        gen.write_string(&with_cr, &mut out);
        // The output should have CR escaped
        assert!(out.contains("\\r"), "Expected \\r escape, got: {:?}", out);
    }

    #[test]
    fn test_string_escape_newline() {
        let gen = Generator::new();
        let mut out = String::new();
        let with_nl = String::from("line") + &"\n" + "break";
        assert!(with_nl.contains('\n'));

        gen.write_string(&with_nl, &mut out);
        assert!(out.contains("\\n"), "Expected \\n escape, got: {:?}", out);
    }

    #[test]
    fn test_string_escape_tab() {
        let gen = Generator::new();
        let mut out = String::new();
        let with_tab = String::from("line") + &"\t" + "break";
        assert!(with_tab.contains('\t'));

        gen.write_string(&with_tab, &mut out);
        assert!(out.contains("\\t"), "Expected \\t escape, got: {:?}", out);
    }

    #[test]
    fn test_string_escape_backslash() {
        let gen = Generator::new();
        let mut out = String::new();
        // Create a string with actual backslash character
        let with_backslash = String::from("path") + &"\\" + "file";
        assert!(with_backslash.contains('\\'));

        gen.write_string(&with_backslash, &mut out);
        // The output should have backslash escaped
        assert!(
            out.contains("\\\\"),
            "Expected \\\\ backslash escape, got: {:?}",
            out
        );
    }

    #[test]
    fn test_generate_block_with_object_value() {
        let mut attrs = HashMap::new();
        attrs.insert(
            "obj".to_string(),
            Value::Object(vec![("key".to_string(), Value::String("val".to_string()))]),
        );

        let block = Block {
            name: "config".to_string(),
            labels: vec![],
            blocks: vec![],
            attributes: attrs,
        };

        let doc = Document {
            blocks: vec![block],
        };
        let output = generate(&doc);
        assert!(output.contains("obj = {key = val}"));
    }

    #[test]
    fn test_generate_nested_block_attrs_and_blocks() {
        let inner = Block {
            name: "inner".to_string(),
            labels: vec![],
            blocks: vec![],
            attributes: vec![("key".to_string(), Value::String("val".to_string()))]
                .into_iter()
                .collect(),
        };

        let block = Block {
            name: "outer".to_string(),
            labels: vec![],
            blocks: vec![inner],
            attributes: vec![("attr".to_string(), Value::Number(1.0))]
                .into_iter()
                .collect(),
        };

        let doc = Document {
            blocks: vec![block],
        };
        let output = generate(&doc);
        assert!(output.contains("outer"));
        assert!(output.contains("inner"));
        assert!(output.contains("attr"));
    }

    #[test]
    fn test_generate_block_with_braces() {
        let mut attrs = HashMap::new();
        attrs.insert("name".to_string(), Value::String("test".to_string()));
        attrs.insert("count".to_string(), Value::Number(42.0));

        let block = Block {
            name: "config".to_string(),
            labels: Vec::new(),
            blocks: Vec::new(),
            attributes: attrs,
        };

        let doc = Document {
            blocks: vec![block],
        };
        let output = generate(&doc);
        assert!(output.contains("config {"));
        assert!(output.contains("name = test"));
        assert!(output.contains("count = 42"));
    }

    #[test]
    fn test_generate_block_with_labels() {
        let mut attrs = HashMap::new();
        attrs.insert("api_key".to_string(), Value::String("sk-xxx".to_string()));

        let block = Block {
            name: "provider".to_string(),
            labels: vec!["openai".to_string()],
            blocks: Vec::new(),
            attributes: attrs,
        };

        let doc = Document {
            blocks: vec![block],
        };
        let output = generate(&doc);
        assert!(output.contains("provider openai"));
        assert!(output.contains("api_key = sk-xxx"));
    }

    #[test]
    fn test_generate_nested_blocks() {
        let inner_block = Block {
            name: "inner".to_string(),
            labels: vec![],
            blocks: Vec::new(),
            attributes: vec![("key".to_string(), Value::String("val".to_string()))]
                .into_iter()
                .collect(),
        };

        let block = Block {
            name: "outer".to_string(),
            labels: vec![],
            blocks: vec![inner_block],
            attributes: HashMap::new(),
        };

        let doc = Document {
            blocks: vec![block],
        };
        let output = generate(&doc);
        assert!(output.contains("outer"));
        assert!(output.contains("inner"));
    }

    #[test]
    fn test_generate_boolean_values() {
        let mut attrs = HashMap::new();
        attrs.insert("enabled".to_string(), Value::Bool(true));
        attrs.insert("disabled".to_string(), Value::Bool(false));

        let block = Block {
            name: "config".to_string(),
            labels: vec![],
            blocks: Vec::new(),
            attributes: attrs,
        };

        let doc = Document {
            blocks: vec![block],
        };
        let output = generate(&doc);
        assert!(output.contains("enabled = true"));
        assert!(output.contains("disabled = false"));
    }

    #[test]
    fn test_generate_null_value() {
        let mut attrs = HashMap::new();
        attrs.insert("value".to_string(), Value::Null);

        let block = Block {
            name: "config".to_string(),
            labels: vec![],
            blocks: Vec::new(),
            attributes: attrs,
        };

        let doc = Document {
            blocks: vec![block],
        };
        let output = generate(&doc);
        assert!(output.contains("value = null"));
    }

    #[test]
    fn test_generate_list_values() {
        let mut attrs = HashMap::new();
        attrs.insert(
            "items".to_string(),
            Value::List(vec![
                Value::Number(1.0),
                Value::Number(2.0),
                Value::Number(3.0),
            ]),
        );

        let block = Block {
            name: "config".to_string(),
            labels: vec![],
            blocks: Vec::new(),
            attributes: attrs,
        };

        let doc = Document {
            blocks: vec![block],
        };
        let output = generate(&doc);
        assert!(output.contains("items = [1, 2, 3]"));
    }

    #[test]
    fn test_generate_string_list() {
        let mut attrs = HashMap::new();
        attrs.insert(
            "names".to_string(),
            Value::List(vec![
                Value::String("a".to_string()),
                Value::String("b".to_string()),
            ]),
        );

        let block = Block {
            name: "config".to_string(),
            labels: vec![],
            blocks: Vec::new(),
            attributes: attrs,
        };

        let doc = Document {
            blocks: vec![block],
        };
        let output = generate(&doc);
        assert!(output.contains("names"));
        assert!(output.contains("a"));
        assert!(output.contains("b"));
    }

    #[test]
    fn test_generate_empty_list() {
        let mut attrs = HashMap::new();
        attrs.insert("items".to_string(), Value::List(vec![]));

        let block = Block {
            name: "config".to_string(),
            labels: vec![],
            blocks: Vec::new(),
            attributes: attrs,
        };

        let doc = Document {
            blocks: vec![block],
        };
        let output = generate(&doc);
        assert!(output.contains("items = []"));
    }

    #[test]
    fn test_generate_object_value() {
        let mut attrs = HashMap::new();
        attrs.insert(
            "obj".to_string(),
            Value::Object(vec![
                ("key1".to_string(), Value::String("val1".to_string())),
                ("key2".to_string(), Value::Number(42.0)),
            ]),
        );

        let block = Block {
            name: "config".to_string(),
            labels: vec![],
            blocks: Vec::new(),
            attributes: attrs,
        };

        let doc = Document {
            blocks: vec![block],
        };
        let output = generate(&doc);
        assert!(output.contains("obj"));
        assert!(output.contains("key1"));
        assert!(output.contains("key2"));
    }

    #[test]
    fn test_generate_multiple_blocks() {
        let block1 = Block {
            name: "a".to_string(),
            labels: vec![],
            blocks: Vec::new(),
            attributes: vec![("x".to_string(), Value::Number(1.0))]
                .into_iter()
                .collect(),
        };
        let block2 = Block {
            name: "b".to_string(),
            labels: vec![],
            blocks: Vec::new(),
            attributes: vec![("y".to_string(), Value::Number(2.0))]
                .into_iter()
                .collect(),
        };

        let doc = Document {
            blocks: vec![block1, block2],
        };
        let output = generate(&doc);
        assert!(output.contains("a {"));
        assert!(output.contains("b {"));
    }

    #[test]
    fn test_generate_from_map() {
        let mut data = HashMap::new();
        data.insert(
            "key".to_string(),
            Value::Object(
                vec![("nested".to_string(), Value::String("value".to_string()))]
                    .into_iter()
                    .collect(),
            ),
        );

        let output = generate_from_map(&data);
        assert!(output.contains("key"));
        assert!(output.contains("value"));
    }

    #[test]
    fn test_generate_from_map_non_object() {
        let mut data = HashMap::new();
        // Insert a non-Object value to trigger the else branch
        data.insert("string_key".to_string(), Value::String("test".to_string()));
        data.insert("number_key".to_string(), Value::Number(42.0));

        let output = generate_from_map(&data);
        assert!(output.contains("string_key"));
        assert!(output.contains("_value"));
    }

    #[test]
    fn test_generator_with_config() {
        let config = GeneratorConfig {
            indent: "\t",
            comments: true,
            labels_as_attrs: false,
        };
        let gen = Generator::with_config(config);
        assert_eq!(gen.config.indent, "\t");
        assert!(gen.config.comments);
        assert!(!gen.config.labels_as_attrs);
    }

    #[test]
    fn test_generator_default() {
        let gen = Generator::default();
        assert_eq!(gen.config.indent, "  ");
        assert!(!gen.config.comments);
    }

    #[test]
    fn test_generate_string_with_special_chars() {
        let gen = Generator::new();
        let mut out = String::new();
        gen.write_string("true", &mut out);
        assert_eq!(out, "\"true\"");

        out.clear();
        gen.write_string("false", &mut out);
        assert_eq!(out, "\"false\"");

        out.clear();
        gen.write_string("null", &mut out);
        assert_eq!(out, "\"null\"");

        out.clear();
        gen.write_string("123", &mut out);
        assert_eq!(out, "\"123\"");

        out.clear();
        gen.write_string(".hello", &mut out);
        assert_eq!(out, "\".hello\"");

        out.clear();
        gen.write_string("-test", &mut out);
        assert_eq!(out, "\"-test\"");
    }

    #[test]
    fn test_generate_empty_block() {
        let block = Block {
            name: "empty".to_string(),
            labels: vec![],
            blocks: Vec::new(),
            attributes: HashMap::new(),
        };

        let doc = Document {
            blocks: vec![block],
        };
        let output = generate(&doc);
        assert!(output.contains("empty"));
    }

    #[test]
    fn test_generate_block_sorted_attrs() {
        let mut attrs = HashMap::new();
        attrs.insert("z".to_string(), Value::Number(1.0));
        attrs.insert("a".to_string(), Value::Number(2.0));
        attrs.insert("m".to_string(), Value::Number(3.0));

        let block = Block {
            name: "config".to_string(),
            labels: vec![],
            blocks: Vec::new(),
            attributes: attrs,
        };

        let doc = Document {
            blocks: vec![block],
        };
        let output = generate(&doc);
        // Check that attributes are sorted alphabetically
        let a_pos = output.find("a = 2").expect("should have a = 2");
        let m_pos = output.find("m = 3").expect("should have m = 3");
        let z_pos = output.find("z = 1").expect("should have z = 1");
        assert!(a_pos < m_pos);
        assert!(m_pos < z_pos);
    }

    #[test]
    fn test_generate_float_numbers() {
        let mut attrs = HashMap::new();
        attrs.insert("pi".to_string(), Value::Number(3.14159));
        attrs.insert("e".to_string(), Value::Number(2.71828));

        let block = Block {
            name: "config".to_string(),
            labels: vec![],
            blocks: Vec::new(),
            attributes: attrs,
        };

        let doc = Document {
            blocks: vec![block],
        };
        let output = generate(&doc);
        assert!(output.contains("pi = 3.14159"));
        assert!(output.contains("e = 2.71828"));
    }

    #[test]
    fn test_generate_integer_as_integer() {
        let mut attrs = HashMap::new();
        attrs.insert("count".to_string(), Value::Number(42.0));

        let block = Block {
            name: "config".to_string(),
            labels: vec![],
            blocks: Vec::new(),
            attributes: attrs,
        };

        let doc = Document {
            blocks: vec![block],
        };
        let output = generate(&doc);
        // Should output as integer without decimal
        assert!(output.contains("count = 42"));
        assert!(!output.contains("count = 42.0"));
    }

    #[test]
    fn test_generate_function_call() {
        let mut attrs = HashMap::new();
        attrs.insert(
            "api_key".to_string(),
            Value::Call(
                "env".to_string(),
                vec![Value::String("API_KEY".to_string())],
            ),
        );

        let block = Block {
            name: "config".to_string(),
            labels: vec![],
            blocks: Vec::new(),
            attributes: attrs,
        };

        let doc = Document {
            blocks: vec![block],
        };
        let output = generate(&doc);
        // API_KEY doesn't need quotes since it has no special chars
        assert!(output.contains("api_key = env(API_KEY)"));
    }

    #[test]
    fn test_generate_function_call_multiple_args() {
        let mut attrs = HashMap::new();
        attrs.insert(
            "path".to_string(),
            Value::Call(
                "concat".to_string(),
                vec![
                    Value::String("hello".to_string()),
                    Value::String("world".to_string()),
                ],
            ),
        );

        let block = Block {
            name: "config".to_string(),
            labels: vec![],
            blocks: Vec::new(),
            attributes: attrs,
        };

        let doc = Document {
            blocks: vec![block],
        };
        let output = generate(&doc);
        // hello and world don't need quotes
        assert!(output.contains("path = concat(hello, world)"));
    }

    #[test]
    fn test_generate_function_call_empty_args() {
        let mut attrs = HashMap::new();
        attrs.insert("val".to_string(), Value::Call("getenv".to_string(), vec![]));

        let block = Block {
            name: "config".to_string(),
            labels: vec![],
            blocks: Vec::new(),
            attributes: attrs,
        };

        let doc = Document {
            blocks: vec![block],
        };
        let output = generate(&doc);
        assert!(output.contains("val = getenv()"));
    }

    #[test]
    fn test_generate_labels_as_attrs() {
        // Test the labels_as_attrs config option
        let mut attrs = HashMap::new();
        attrs.insert("api_key".to_string(), Value::String("sk-xxx".to_string()));

        let inner = Block {
            name: "models".to_string(),
            labels: vec!["kimi-k2.5".to_string()],
            blocks: vec![],
            attributes: vec![("id".to_string(), Value::String("kimi-k2.5".to_string()))]
                .into_iter()
                .collect(),
        };

        let block = Block {
            name: "providers".to_string(),
            labels: vec!["openai".to_string()],
            blocks: vec![inner],
            attributes: attrs,
        };

        let doc = Document {
            blocks: vec![block],
        };

        // Without labels_as_attrs (default ACL format)
        let output = generate(&doc);
        // ACL format uses unquoted labels when not needed
        assert!(
            output.contains("providers openai"),
            "Should have ACL-style label"
        );
        assert!(
            output.contains("models kimi-k2.5"),
            "Should have ACL-style label"
        );

        // With labels_as_attrs (HCL format)
        let config = GeneratorConfig {
            labels_as_attrs: true,
            ..Default::default()
        };
        let gen = Generator::with_config(config);
        let output = gen.generate(&doc);
        assert!(
            output.contains("providers {"),
            "Should have HCL-style block"
        );
        assert!(
            output.contains("name = openai"),
            "Should output label as name attr"
        );
        assert!(
            !output.contains("providers openai"),
            "Should NOT have ACL-style label header"
        );
    }

    #[test]
    fn test_generate_labels_as_attrs_nested() {
        // Test nested blocks with labels_as_attrs
        let model_block = Block {
            name: "models".to_string(),
            labels: vec!["kimi-k2.5".to_string()],
            blocks: vec![],
            attributes: vec![
                ("id".to_string(), Value::String("kimi-k2.5".to_string())),
                ("name".to_string(), Value::String("Kimixxx".to_string())),
            ]
            .into_iter()
            .collect(),
        };

        let provider_block = Block {
            name: "providers".to_string(),
            labels: vec!["openai".to_string()],
            blocks: vec![model_block],
            attributes: vec![
                ("name".to_string(), Value::String("openai".to_string())),
                (
                    "base_url".to_string(),
                    Value::String("https://api.openai.com/v1".to_string()),
                ),
            ]
            .into_iter()
            .collect(),
        };

        let doc = Document {
            blocks: vec![provider_block],
        };

        let config = GeneratorConfig {
            labels_as_attrs: true,
            ..Default::default()
        };
        let gen = Generator::with_config(config);
        let output = gen.generate(&doc);

        // Should NOT have labels in block header (e.g., "providers openai")
        assert!(
            !output.contains("providers openai"),
            "Should not have ACL-style label in header"
        );
        assert!(
            !output.contains("models kimi-k2.5"),
            "Should not have ACL-style label in header"
        );

        // Should have HCL-style blocks
        assert!(output.contains("providers {"));
        assert!(output.contains("models {"));

        // Should have name = ... as attribute (unquoted for simple identifiers)
        assert!(output.contains("name = openai"));
        assert!(output.contains("name = kimi-k2.5"));
    }
}
