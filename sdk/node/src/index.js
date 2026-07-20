/**
 * ACL SDK - Agent Configuration Language Parser and Generator
 *
 * @module acl
 */

// Lexer
const { Lexer } = require('./lexer/Lexer.js');

// Diagnostics
const {
  DIAGNOSTIC_CODES,
  ParseError,
} = require('./diagnostic/ParseError.js');

// Parser
const {
  collectDiagnostics,
  DEFAULT_PARSE_LIMITS,
  Parser,
  parse,
} = require('./parser/Parser.js');

// Declarative schema validation
const {
  SCHEMA_DIAGNOSTIC_CODES,
  validateDocument,
} = require('./schema/schema.js');

// Canonical bytes and digests
const {
  CANONICAL_DIGEST_ALGORITHM,
  CanonicalError,
  canonicalBytes,
  canonicalDigest,
} = require('./canonical/canonical.js');

// Generator
const { generate } = require('./generator/generator.js');

// Builder
const {
  string,
  number,
  boolean,
  bool,
  nullValue,
  list,
  call,
  BlockBuilder,
  DocumentBuilder,
} = require('./builder/builder.js');

module.exports = {
  // Core parsing
  parse,
  collectDiagnostics,
  DEFAULT_PARSE_LIMITS,
  DIAGNOSTIC_CODES,
  ParseError,
  validateDocument,
  SCHEMA_DIAGNOSTIC_CODES,

  // Canonical bytes and digests
  CANONICAL_DIGEST_ALGORITHM,
  CanonicalError,
  canonicalBytes,
  canonicalDigest,

  // Generation
  generate,

  // Value constructors
  string,
  number,
  boolean,
  bool,
  nullValue,
  list,
  call,

  // Builders
  BlockBuilder,
  DocumentBuilder,

  // Lexer class for advanced usage
  Lexer,

  // Parser class for advanced usage
  Parser,
};
