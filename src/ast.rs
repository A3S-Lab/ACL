// ============================================================================
// ACL AST - Abstract Syntax Tree for Agent Configuration Language
// ============================================================================

use std::collections::HashMap;

/// An ACL configuration document
#[derive(Debug, Clone, Default)]
pub struct Document {
    pub blocks: Vec<Block>,
}

/// A named block like `providers { }` or `models { }`
#[derive(Debug, Clone)]
pub struct Block {
    /// Block type name (e.g., "providers", "models")
    pub name: String,
    /// Optional block labels (e.g., `"openai"` in `providers "openai" { }`)
    pub labels: Vec<String>,
    /// Nested blocks inside this block
    pub blocks: Vec<Block>,
    /// Attribute assignments
    pub attributes: HashMap<String, Value>,
}

/// A value in ACL (string, number, bool, list, etc.)
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// A string value: "hello"
    String(String),
    /// A number value: 42 or 3.14
    Number(f64),
    /// A boolean value: true or false
    Bool(bool),
    /// A list value: [1, 2, 3]
    List(Vec<Value>),
    /// An object/block value: { key = "value" }
    Object(Vec<(String, Value)>),
    /// Null value
    Null,
    /// A function call: env("VAR") or concat("a", "b")
    Call(String, Vec<Value>),
}

impl Value {
    /// Get the string value if this is a String variant
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// Get the number value if this is a Number variant
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }

    /// Get the bool value if this is a Bool variant
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Check if this is a Null value
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Check if this is a string value
    pub fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::String(s) => write!(f, "{}", s),
            Value::Number(n) => {
                if n.fract() == 0.0 {
                    write!(f, "{}", *n as i64)
                } else {
                    write!(f, "{}", n)
                }
            }
            Value::Bool(b) => write!(f, "{}", b),
            Value::List(items) => {
                let items_str: Vec<String> = items.iter().map(|v| v.to_string()).collect();
                write!(f, "[{}]", items_str.join(", "))
            }
            Value::Object(pairs) => {
                let pairs_str: Vec<String> = pairs
                    .iter()
                    .map(|(k, v)| format!("{} = {}", k, v))
                    .collect();
                write!(f, "{{{}}}", pairs_str.join(", "))
            }
            Value::Null => write!(f, "null"),
            Value::Call(name, args) => {
                let args_str: Vec<String> = args.iter().map(|v| v.to_string()).collect();
                write!(f, "{}({})", name, args_str.join(", "))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_as_str() {
        assert_eq!(Value::String("hello".to_string()).as_str(), Some("hello"));
        assert_eq!(Value::Number(42.0).as_str(), None);
        assert_eq!(Value::Bool(true).as_str(), None);
        assert_eq!(Value::Null.as_str(), None);
    }

    #[test]
    fn test_value_as_number() {
        assert_eq!(Value::Number(42.0).as_number(), Some(42.0));
        assert_eq!(Value::String("hello".to_string()).as_number(), None);
        assert_eq!(Value::Bool(true).as_number(), None);
        assert_eq!(Value::Null.as_number(), None);
    }

    #[test]
    fn test_value_as_bool() {
        assert_eq!(Value::Bool(true).as_bool(), Some(true));
        assert_eq!(Value::Bool(false).as_bool(), Some(false));
        assert_eq!(Value::String("hello".to_string()).as_bool(), None);
        assert_eq!(Value::Number(42.0).as_bool(), None);
    }

    #[test]
    fn test_value_is_null() {
        assert!(Value::Null.is_null());
        assert!(!Value::String("hello".to_string()).is_null());
        assert!(!Value::Number(42.0).is_null());
        assert!(!Value::Bool(true).is_null());
    }

    #[test]
    fn test_value_is_string() {
        assert!(Value::String("hello".to_string()).is_string());
        assert!(!Value::Number(42.0).is_string());
        assert!(!Value::Bool(true).is_string());
        assert!(!Value::Null.is_string());
    }

    #[test]
    fn test_value_display() {
        assert_eq!(Value::String("hello".to_string()).to_string(), "hello");
        assert_eq!(Value::Number(42.0).to_string(), "42");
        assert_eq!(Value::Number(3.14).to_string(), "3.14");
        assert_eq!(Value::Bool(true).to_string(), "true");
        assert_eq!(Value::Bool(false).to_string(), "false");
        assert_eq!(Value::Null.to_string(), "null");
        assert_eq!(Value::List(vec![]).to_string(), "[]");
        assert_eq!(
            Value::List(vec![Value::Number(1.0), Value::Number(2.0)]).to_string(),
            "[1, 2]"
        );
        assert_eq!(Value::Object(vec![]).to_string(), "{}");
        assert_eq!(
            Value::Object(vec![("a".to_string(), Value::Number(1.0))]).to_string(),
            "{a = 1}"
        );
    }

    #[test]
    fn test_document_default() {
        let doc = Document::default();
        assert!(doc.blocks.is_empty());
    }

    #[test]
    fn test_block_clone() {
        let mut attrs = HashMap::new();
        attrs.insert("key".to_string(), Value::String("value".to_string()));
        let block = Block {
            name: "test".to_string(),
            labels: vec!["label".to_string()],
            blocks: vec![],
            attributes: attrs,
        };
        let cloned = block.clone();
        assert_eq!(cloned.name, "test");
        assert_eq!(cloned.labels, vec!["label"]);
    }

    #[test]
    fn test_value_clone() {
        let value = Value::String("test".to_string());
        let cloned = value.clone();
        assert_eq!(value, cloned);
    }

    #[test]
    fn test_value_partial_eq() {
        assert_eq!(
            Value::String("a".to_string()),
            Value::String("a".to_string())
        );
        assert_ne!(
            Value::String("a".to_string()),
            Value::String("b".to_string())
        );
        assert_eq!(Value::Number(42.0), Value::Number(42.0));
        assert_ne!(Value::Number(42.0), Value::Number(43.0));
        assert_eq!(Value::Bool(true), Value::Bool(true));
        assert_ne!(Value::Bool(true), Value::Bool(false));
        assert_eq!(Value::Null, Value::Null);
    }
}
