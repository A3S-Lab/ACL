/**
 * ACL SDK Type Definitions
 * Agent Configuration Language - Parser and Generator
 */

export interface Location {
  line: number;
  column: number;
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

export interface ParseError {
  message: string;
  line: number;
  column: number;
}

// Core API
export function parse(input: string): Document;
export function generate(doc: Document): string;

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
  constructor(input: string);
  tokenize(): Token[];
}

// Parser class (advanced usage)
export class Parser {
  constructor();
  parse(input: string): Document;
}

// Default export with all members
export default {
  parse,
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
