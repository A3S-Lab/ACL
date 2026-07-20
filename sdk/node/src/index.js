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
const { DEFAULT_PARSE_LIMITS, Parser, parse } = require('./parser/Parser.js');

// Canonical bytes and digests
const {
  CANONICAL_DIGEST_ALGORITHM,
  CanonicalError,
  canonicalBytes,
  canonicalDigest,
} = require('./canonical/canonical.js');

// Generator
const { generate, generateHCL } = require('./generator/generator.js');

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
  DEFAULT_PARSE_LIMITS,
  DIAGNOSTIC_CODES,
  ParseError,

  // Canonical bytes and digests
  CANONICAL_DIGEST_ALGORITHM,
  CanonicalError,
  canonicalBytes,
  canonicalDigest,

  // Generation
  generate,
  generateHCL,

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
