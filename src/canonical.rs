use crate::ast::{Block, Document, Value};
use crate::generator::generate;
use crate::schema::{BlockSchema, Schema};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
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
    Ok(canonical_bytes_from_validated(document))
}

/// Returns canonical UTF-8 after recursively sorting schema-declared
/// unordered block occurrences.
///
/// This function uses the schema only as normalization metadata. Call
/// [`crate::validate_document`] first when schema admission is required.
pub fn canonical_bytes_with_schema(
    document: &Document,
    schema: &Schema,
) -> Result<Vec<u8>, CanonicalError> {
    validate_document(document)?;
    let normalized = normalize_document(document, schema);
    Ok(canonical_bytes_from_validated(&normalized))
}

fn canonical_bytes_from_validated(document: &Document) -> Vec<u8> {
    let mut canonical = generate(document);
    while canonical.ends_with('\n') {
        canonical.pop();
    }
    canonical.push('\n');
    canonical.into_bytes()
}

/// Returns lowercase `sha256:<hex>` over [`canonical_bytes`].
pub fn canonical_digest(document: &Document) -> Result<String, CanonicalError> {
    let bytes = canonical_bytes(document)?;
    Ok(digest_bytes(&bytes))
}

/// Returns lowercase `sha256:<hex>` over
/// [`canonical_bytes_with_schema`].
pub fn canonical_digest_with_schema(
    document: &Document,
    schema: &Schema,
) -> Result<String, CanonicalError> {
    let bytes = canonical_bytes_with_schema(document, schema)?;
    Ok(digest_bytes(&bytes))
}

fn digest_bytes(bytes: &[u8]) -> String {
    format!("{CANONICAL_DIGEST_ALGORITHM}:{:x}", Sha256::digest(bytes))
}

fn normalize_document(document: &Document, schema: &Schema) -> Document {
    Document {
        blocks: normalize_blocks(&document.blocks, schema, true),
    }
}

fn normalize_blocks(blocks: &[Block], schema: &Schema, document_root: bool) -> Vec<Block> {
    let mut normalized = Vec::with_capacity(blocks.len());
    let mut unordered_positions: BTreeMap<String, Vec<usize>> = BTreeMap::new();

    for (index, block) in blocks.iter().enumerate() {
        let is_root_attribute = document_root && bare_attribute(block);
        let rule = if is_root_attribute {
            None
        } else {
            schema.block_rule(&block.name)
        };
        normalized.push(match rule {
            Some(rule) => normalize_block(block, rule),
            None => block.clone(),
        });
        if rule.is_some_and(BlockSchema::is_unordered) {
            unordered_positions
                .entry(block.name.clone())
                .or_default()
                .push(index);
        }
    }

    for positions in unordered_positions.into_values() {
        if positions.len() < 2 {
            continue;
        }
        let mut matching = positions
            .iter()
            .map(|index| normalized[*index].clone())
            .collect::<Vec<_>>();
        matching.sort_by_cached_key(canonical_block_key);
        for (index, block) in positions.into_iter().zip(matching) {
            normalized[index] = block;
        }
    }
    normalized
}

fn normalize_block(block: &Block, schema: &BlockSchema) -> Block {
    Block {
        name: block.name.clone(),
        labels: block.labels.clone(),
        blocks: normalize_blocks(&block.blocks, schema.body_schema(), false),
        attributes: block.attributes.clone(),
    }
}

fn canonical_block_key(block: &Block) -> Vec<u8> {
    canonical_bytes_from_validated(&Document {
        blocks: vec![block.clone()],
    })
}

fn bare_attribute(block: &Block) -> bool {
    block.labels.is_empty()
        && block.blocks.is_empty()
        && block.attributes.len() == 1
        && block.attributes.contains_key(&block.name)
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
