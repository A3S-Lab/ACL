/**
 * ACL SDK Type Definitions
 * Agent Configuration Language - Parser and Generator
 */

export interface Location {
  line: number;
  column: number;
  /** Zero-based UTF-8 byte offset from the start of the document. */
  offset: number;
}

export interface Span {
  start: Location;
  end: Location;
}

export type TokenType =
  | 'LeftBrace' | 'RightBrace'
  | 'LeftBracket' | 'RightBracket'
  | 'LeftParen' | 'RightParen'
  | 'Equal' | 'PlusEqual' | 'Colon' | 'Comma'
  | 'Comment' | 'Newline'
  | 'Ident' | 'String' | 'Number'
  | 'True' | 'False' | 'Null' | 'Eof';

export interface Token {
  type: TokenType;
  value?: string | number;
  span: Span;
}

export type Value =
  | { kind: 'String'; value: string }
  | { kind: 'Number'; value: number }
  | { kind: 'Bool'; value: boolean }
  | { kind: 'List'; items: Value[] }
  | { kind: 'Object'; pairs: [string, Value][] }
  | { kind: 'Null' }
  | { kind: 'Call'; name: string; args: Value[] };

export interface Block {
  name: string;
  labels: string[];
  blocks: Block[];
  attributes: Map<string, Value>;
}

export interface Document {
  blocks: Block[];
}

export type DiagnosticCode =
  | 'acl.limit.document_bytes'
  | 'acl.limit.token_bytes'
  | 'acl.limit.nesting_depth'
  | 'acl.limit.collection_items'
  | 'acl.parse.unexpected_token'
  | 'acl.parse.expected_token'
  | 'acl.parse.unexpected_eof';

export const DIAGNOSTIC_CODES: Readonly<{
  DOCUMENT_BYTES_LIMIT: 'acl.limit.document_bytes';
  TOKEN_BYTES_LIMIT: 'acl.limit.token_bytes';
  NESTING_DEPTH_LIMIT: 'acl.limit.nesting_depth';
  COLLECTION_ITEMS_LIMIT: 'acl.limit.collection_items';
  UNEXPECTED_TOKEN: 'acl.parse.unexpected_token';
  EXPECTED_TOKEN: 'acl.parse.expected_token';
  UNEXPECTED_EOF: 'acl.parse.unexpected_eof';
}>;

export class ParseError extends Error {
  readonly code: DiagnosticCode;
  readonly span: Span;
  readonly line: number;
  readonly column: number;
}

export type CanonicalErrorCode =
  | 'acl.canonical.invalid_unicode'
  | 'acl.canonical.non_finite_number'
  | 'acl.canonical.unsupported_identifier';

export class CanonicalError extends Error {
  readonly code: CanonicalErrorCode;
}

export interface ParseLimits {
  maxDocumentBytes: number;
  maxNestingDepth: number;
  maxCollectionItems: number;
  maxTokenBytes: number;
  maxDiagnostics: number;
}

export interface DiagnosticReport {
  readonly diagnostics: ParseError[];
  readonly truncated: boolean;
}

// Core API
export const DEFAULT_PARSE_LIMITS: Readonly<ParseLimits>;
export const CANONICAL_DIGEST_ALGORITHM: 'sha256';
export function parse(input: string, limits?: Partial<ParseLimits>): Document;
export function collectDiagnostics(input: string, limits?: Partial<ParseLimits>): DiagnosticReport;
export function generate(doc: Document): string;
export function canonicalBytes(doc: Document): Uint8Array;
export function canonicalDigest(doc: Document): string;

// Value constructors
export function string(s: string): Value;
export function number(n: number): Value;
export function boolean(b: boolean): Value;
export function bool(b: boolean): Value;
export function nullValue(): Value;
export function list(items: Value[]): Value;
export function call(name: string, args: Value[]): Value;

// Builders
export class BlockBuilder {
  constructor(name: string);
  label(label: string): BlockBuilder;
  attr(key: string, value: Value): BlockBuilder;
  nestedBlock(block: Block): BlockBuilder;
  build(): Block;
}

export class DocumentBuilder {
  constructor();
  block(block: Block): DocumentBuilder;
  kvBlock(name: string, key: string, value: Value): DocumentBuilder;
  build(): Document;
}

// Lexer (advanced usage)
export class Lexer {
  constructor(input: string, maxTokenBytes?: number);
  tokenize(): Token[];
}

// Parser class (advanced usage)
export class Parser {
  constructor();
  parse(input: string, limits?: Partial<ParseLimits>): Document;
  collectDiagnostics(input: string, limits?: Partial<ParseLimits>): DiagnosticReport;
}

// Default export with all members
export default {
  parse,
  collectDiagnostics,
  DEFAULT_PARSE_LIMITS,
  DIAGNOSTIC_CODES,
  ParseError,
  CANONICAL_DIGEST_ALGORITHM,
  CanonicalError,
  canonicalBytes,
  canonicalDigest,
  generate,
  string,
  number,
  boolean,
  bool,
  nullValue,
  list,
  call,
  BlockBuilder,
  DocumentBuilder,
  Lexer,
  Parser,
};
