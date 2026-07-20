const { createHash } = require('node:crypto');
const { generate } = require('../generator/generator.js');

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
  const generated = generate(document).replace(/\n+$/u, '');
  return Buffer.from(`${generated}\n`, 'utf8');
}

function canonicalDigest(document) {
  const bytes = canonicalBytes(document);
  return `${CANONICAL_DIGEST_ALGORITHM}:${createHash(CANONICAL_DIGEST_ALGORITHM)
    .update(bytes)
    .digest('hex')}`;
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
  canonicalDigest,
};
