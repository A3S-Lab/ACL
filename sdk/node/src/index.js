/**
 * ACL SDK - Agent Configuration Language Parser and Generator
 *
 * @module acl
 */

// Lexer
const { Lexer } = require('./lexer/Lexer.js');

// Parser
const { Parser, parse } = require('./parser/Parser.js');

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
