const { createHash } = require('node:crypto');
const { generate } = require('../generator/generator.js');
const { normalizeSchema } = require('../schema/definition.js');

const CANONICAL_DIGEST_ALGORITHM = 'sha256';

class CanonicalError extends Error {
  constructor(code, message) {
    super(message);
    this.name = 'CanonicalError';
    this.code = code;
  }
}

function canonicalBytes(document) {
  validateDocument(document);
  return canonicalBytesFromValidated(document);
}

function canonicalBytesWithSchema(document, schema) {
  const normalizedSchema = normalizeSchema(schema, '$schema', new Set());
  validateDocument(document);
  return canonicalBytesFromValidated(normalizeDocument(document, normalizedSchema));
}

function canonicalBytesFromValidated(document) {
  const generated = generate(document).replace(/\n+$/u, '');
  return Buffer.from(`${generated}\n`, 'utf8');
}

function canonicalDigest(document) {
  const bytes = canonicalBytes(document);
  return digestBytes(bytes);
}

function canonicalDigestWithSchema(document, schema) {
  const bytes = canonicalBytesWithSchema(document, schema);
  return digestBytes(bytes);
}

function digestBytes(bytes) {
  return `${CANONICAL_DIGEST_ALGORITHM}:${createHash(CANONICAL_DIGEST_ALGORITHM)
    .update(bytes)
    .digest('hex')}`;
}

function normalizeDocument(document, schema) {
  return {
    blocks: normalizeBlocks(document.blocks, schema, true),
  };
}

function normalizeBlocks(blocks, schema, documentRoot) {
  const normalized = [];
  const unorderedPositions = new Map();

  for (const [index, block] of blocks.entries()) {
    const rootAttribute = documentRoot && bareAttribute(block);
    const rule = rootAttribute ? undefined : schema.blocks.get(block.name);
    normalized.push(rule ? normalizeBlock(block, rule) : block);
    if (rule?.unordered) {
      const positions = unorderedPositions.get(block.name);
      if (positions) positions.push(index);
      else unorderedPositions.set(block.name, [index]);
    }
  }

  for (const positions of unorderedPositions.values()) {
    if (positions.length < 2) continue;
    const matching = positions
      .map((index) => normalized[index])
      .map((block) => ({
        block,
        key: canonicalBytesFromValidated({ blocks: [block] }),
      }))
      .sort((left, right) => Buffer.compare(left.key, right.key))
      .map(({ block }) => block);
    for (const [offset, index] of positions.entries()) {
      normalized[index] = matching[offset];
    }
  }
  return normalized;
}

function normalizeBlock(block, schema) {
  return {
    ...block,
    blocks: normalizeBlocks(block.blocks, schema.body, false),
  };
}

function bareAttribute(block) {
  return block.labels.length === 0
    && block.blocks.length === 0
    && block.attributes.size === 1
    && block.attributes.has(block.name);
}

function validateDocument(document) {
  for (const block of document.blocks) {
    validateBlock(block);
  }
}

function validateBlock(block) {
  validateIdentifier(block.name);
  for (const label of block.labels) {
    validateScalarString(label);
  }
  for (const [key, value] of block.attributes) {
    validateIdentifier(key);
    validateValue(value);
  }
  for (const nested of block.blocks) {
    validateBlock(nested);
  }
}

function validateValue(value) {
  switch (value.kind) {
    case 'Number':
      if (!Number.isFinite(value.value)) {
        throw new CanonicalError(
          'acl.canonical.non_finite_number',
          'ACL canonicalization requires finite numbers'
        );
      }
      break;
    case 'List':
      for (const item of value.items) validateValue(item);
      break;
    case 'Object':
      for (const [key, item] of value.pairs) {
        validateIdentifier(key);
        validateValue(item);
      }
      break;
    case 'Call':
      validateIdentifier(value.name);
      for (const argument of value.args) validateValue(argument);
      break;
    case 'String':
      validateScalarString(value.value);
      break;
    case 'Bool':
    case 'Null':
      break;
  }
}

function validateScalarString(value) {
  for (let index = 0; index < value.length; index += 1) {
    const codeUnit = value.charCodeAt(index);
    if (codeUnit >= 0xD800 && codeUnit <= 0xDBFF) {
      const next = value.charCodeAt(index + 1);
      if (!(next >= 0xDC00 && next <= 0xDFFF)) {
        throw new CanonicalError(
          'acl.canonical.invalid_unicode',
          'ACL canonicalization requires Unicode scalar values'
        );
      }
      index += 1;
    } else if (codeUnit >= 0xDC00 && codeUnit <= 0xDFFF) {
      throw new CanonicalError(
        'acl.canonical.invalid_unicode',
        'ACL canonicalization requires Unicode scalar values'
      );
    }
  }
}

function validateIdentifier(identifier) {
  if (!/^[A-Za-z_][A-Za-z0-9_]*$/u.test(identifier)) {
    throw new CanonicalError(
      'acl.canonical.unsupported_identifier',
      'ACL canonicalization requires portable ASCII identifiers'
    );
  }
}

module.exports = {
  CANONICAL_DIGEST_ALGORITHM,
  CanonicalError,
  canonicalBytes,
  canonicalBytesWithSchema,
  canonicalDigest,
  canonicalDigestWithSchema,
};
