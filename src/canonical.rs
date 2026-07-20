use crate::ast::{Block, Document, Value};
use crate::generator::generate;
use sha2::{Digest, Sha256};
use std::fmt;

/// Digest algorithm used by [`canonical_digest`].
pub const CANONICAL_DIGEST_ALGORITHM: &str = "sha256";

/// Stable, value-redacting failures produced by ACL canonicalization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanonicalError {
    /// A programmatic AST contained NaN or an infinity.
    NonFiniteNumber,
    /// An AST identifier was outside the portable cross-SDK grammar.
    UnsupportedIdentifier,
}

impl CanonicalError {
    /// Returns the stable cross-SDK error code.
    pub const fn code(self) -> &'static str {
        match self {
            Self::NonFiniteNumber => "acl.canonical.non_finite_number",
            Self::UnsupportedIdentifier => "acl.canonical.unsupported_identifier",
        }
    }
}

impl fmt::Display for CanonicalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteNumber => {
                formatter.write_str("ACL canonicalization requires finite numbers")
            }
            Self::UnsupportedIdentifier => {
                formatter.write_str("ACL canonicalization requires portable ASCII identifiers")
            }
        }
    }
}

impl std::error::Error for CanonicalError {}

/// Returns the canonical UTF-8 representation with exactly one final LF.
pub fn canonical_bytes(document: &Document) -> Result<Vec<u8>, CanonicalError> {
    validate_document(document)?;
    let mut canonical = generate(document);
    while canonical.ends_with('\n') {
        canonical.pop();
    }
    canonical.push('\n');
    Ok(canonical.into_bytes())
}

/// Returns lowercase `sha256:<hex>` over [`canonical_bytes`].
pub fn canonical_digest(document: &Document) -> Result<String, CanonicalError> {
    let bytes = canonical_bytes(document)?;
    Ok(format!(
        "{CANONICAL_DIGEST_ALGORITHM}:{:x}",
        Sha256::digest(bytes)
    ))
}

fn validate_document(document: &Document) -> Result<(), CanonicalError> {
    for block in &document.blocks {
        validate_block(block)?;
    }
    Ok(())
}

fn validate_block(block: &Block) -> Result<(), CanonicalError> {
    validate_identifier(&block.name)?;
    for key in block.attributes.keys() {
        validate_identifier(key)?;
    }
    for value in block.attributes.values() {
        validate_value(value)?;
    }
    for block in &block.blocks {
        validate_block(block)?;
    }
    Ok(())
}

fn validate_value(value: &Value) -> Result<(), CanonicalError> {
    match value {
        Value::Number(number) if !number.is_finite() => Err(CanonicalError::NonFiniteNumber),
        Value::List(values) => {
            for value in values {
                validate_value(value)?;
            }
            Ok(())
        }
        Value::Object(pairs) => {
            for (key, value) in pairs {
                validate_identifier(key)?;
                validate_value(value)?;
            }
            Ok(())
        }
        Value::Call(name, arguments) => {
            validate_identifier(name)?;
            for argument in arguments {
                validate_value(argument)?;
            }
            Ok(())
        }
        Value::String(_) | Value::Number(_) | Value::Bool(_) | Value::Null => Ok(()),
    }
}

fn validate_identifier(identifier: &str) -> Result<(), CanonicalError> {
    let mut bytes = identifier.bytes();
    let Some(first) = bytes.next() else {
        return Err(CanonicalError::UnsupportedIdentifier);
    };
    if !(first.is_ascii_alphabetic() || first == b'_')
        || bytes.any(|byte| !(byte.is_ascii_alphanumeric() || byte == b'_'))
    {
        return Err(CanonicalError::UnsupportedIdentifier);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Block, Document, Value};
    use std::collections::HashMap;

    #[test]
    fn errors_are_stable_and_redacted() {
        assert_eq!(
            CanonicalError::NonFiniteNumber.code(),
            "acl.canonical.non_finite_number"
        );
        assert_eq!(
            CanonicalError::UnsupportedIdentifier.code(),
            "acl.canonical.unsupported_identifier"
        );
        assert!(!CanonicalError::NonFiniteNumber
            .to_string()
            .contains("private"));
    }

    #[test]
    fn rejects_nonportable_programmatic_identifiers() {
        let document = Document {
            blocks: vec![Block {
                name: "配置".into(),
                labels: Vec::new(),
                blocks: Vec::new(),
                attributes: HashMap::from([(
                    "private".into(),
                    Value::String("private-value".into()),
                )]),
            }],
        };

        assert_eq!(
            canonical_bytes(&document),
            Err(CanonicalError::UnsupportedIdentifier)
        );
    }
}
