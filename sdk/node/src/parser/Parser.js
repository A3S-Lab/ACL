/**
 * ACL Parser
 */

const { Lexer } = require('../lexer/Lexer.js');
const {
  DIAGNOSTIC_CODES,
  ParseError,
  pointSpan,
} = require('../diagnostic/ParseError.js');

const DEFAULT_PARSE_LIMITS = Object.freeze({
  maxDocumentBytes: 1024 * 1024,
  maxNestingDepth: 64,
  maxCollectionItems: 10_000,
  maxTokenBytes: 256 * 1024,
  maxDiagnostics: 100,
});

function normalizeParseLimits(limits) {
  if (limits !== undefined && (limits === null || typeof limits !== 'object')) {
    throw new TypeError('ACL parse limits must be an object');
  }

  const normalized = { ...DEFAULT_PARSE_LIMITS, ...(limits ?? {}) };
  for (const name of Object.keys(DEFAULT_PARSE_LIMITS)) {
    const value = normalized[name];
    if (!Number.isSafeInteger(value) || value < 0) {
      throw new TypeError(`ACL parse limit ${name} must be a non-negative safe integer`);
    }
  }
  return normalized;
}

class Parser {
  constructor(tokens) {
    this.tokens = tokens;
    this.pos = 0;
    this.limits = DEFAULT_PARSE_LIMITS;
    this.blockDepth = 0;
  }

  current() {
    return this.tokens[this.pos];
  }

  advance() {
    if (this.pos < this.tokens.length - 1) {
      this.pos++;
    }
  }

  skipNewlines() {
    while (this.current()?.type === 'Newline' || this.current()?.type === 'Comment') {
      this.advance();
    }
  }

  peek(offset = 1) {
    return this.tokens[this.pos + offset];
  }

  eofSpan() {
    return this.tokens[this.tokens.length - 1]?.span ?? pointSpan();
  }

  parse(input, limits) {
    this.prepare(input, limits);
    return this.parseDocument();
  }

  collectDiagnostics(input, limits) {
    const report = { diagnostics: [], truncated: false };
    try {
      this.prepare(input, limits);
    } catch (error) {
      if (!(error instanceof ParseError)) throw error;
      this.recordDiagnostic(report, error);
      return report;
    }
    return this.collectDocumentDiagnostics(report);
  }

  prepare(input, limits) {
    this.limits = normalizeParseLimits(limits);
    if (Buffer.byteLength(input, 'utf8') > this.limits.maxDocumentBytes) {
      throw new ParseError(
        DIAGNOSTIC_CODES.DOCUMENT_BYTES_LIMIT,
        `ACL parse limit exceeded: document is larger than ${this.limits.maxDocumentBytes} bytes`,
        pointSpan()
      );
    }

    const lexer = new Lexer(input, this.limits.maxTokenBytes);
    this.tokens = lexer.tokenize();
    this.pos = 0;
    this.blockDepth = 0;
    this.validateNesting();
  }

  validateNesting() {
    let depth = 0;
    for (const token of this.tokens) {
      if (token.type === 'LeftBrace' || token.type === 'LeftBracket' || token.type === 'LeftParen') {
        depth++;
        if (depth > this.limits.maxNestingDepth) {
          throw new ParseError(
            DIAGNOSTIC_CODES.NESTING_DEPTH_LIMIT,
            `ACL parse limit exceeded: nesting depth is greater than ${this.limits.maxNestingDepth}`,
            token.span
          );
        }
      } else if (token.type === 'RightBrace' || token.type === 'RightBracket' || token.type === 'RightParen') {
        depth = Math.max(0, depth - 1);
      }
    }
  }

  ensureCollectionCapacity(currentLength, token = this.current()) {
    if (currentLength >= this.limits.maxCollectionItems) {
      throw new ParseError(
        DIAGNOSTIC_CODES.COLLECTION_ITEMS_LIMIT,
        `ACL parse limit exceeded: collection has more than ${this.limits.maxCollectionItems} items`,
        token?.span ?? this.eofSpan()
      );
    }
  }

  parseDocument() {
    const blocks = [];
    this.skipNewlines();

    while (this.current()?.type !== 'Eof') {
      this.skipNewlines();
      if (this.current()?.type === 'Eof') break;
      this.parseDocumentItem(blocks);
      this.skipNewlines();
    }

    return { blocks };
  }

  collectDocumentDiagnostics(report) {
    const blocks = [];
    this.skipNewlines();

    while (this.current()?.type !== 'Eof') {
      const itemStart = this.pos;
      try {
        this.parseDocumentItem(blocks);
        this.skipNewlines();
      } catch (error) {
        if (!(error instanceof ParseError)) throw error;
        const isLimit = error.code.startsWith('acl.limit.');
        if (!this.recordDiagnostic(report, error) || isLimit) break;
        this.recoverAfterError(itemStart, error.span.start.line);
      }
    }

    return report;
  }

  parseDocumentItem(blocks) {
    if (this.current()?.type === 'Ident') {
      this.ensureCollectionCapacity(blocks.length);
      const next = this.peek();
      if (next?.type === 'Equal' || next?.type === 'Colon') {
        blocks.push(this.parseBareAttribute());
      } else {
        blocks.push(this.parseBlock());
      }
      return;
    }

    const error = this.current();
    throw new ParseError(
      DIAGNOSTIC_CODES.UNEXPECTED_TOKEN,
      `Unexpected token: ${error?.type ?? 'Eof'}`,
      error?.span ?? this.eofSpan()
    );
  }

  recordDiagnostic(report, error) {
    if (report.diagnostics.length >= this.limits.maxDiagnostics) {
      report.truncated = true;
      return false;
    }
    report.diagnostics.push(error);
    return true;
  }

  recoverAfterError(itemStart, errorLine) {
    if (this.pos === itemStart && this.current()?.type !== 'Eof') {
      this.advance();
    }
    while (
      this.current()?.type !== 'Eof'
      && this.current()?.span.start.line <= errorLine
    ) {
      this.advance();
    }
    this.blockDepth = 0;
    this.skipNewlines();
  }

  parseBareAttribute() {
    const name = this.current().value;
    this.advance(); // consume ident
    this.advance(); // consume = or :
    const value = this.parseValue();
    return {
      name,
      labels: [],
      blocks: [],
      attributes: new Map([[name, value]]),
    };
  }

  parseBlock() {
    const token = this.current();
    if (this.blockDepth >= this.limits.maxNestingDepth) {
      throw new ParseError(
        DIAGNOSTIC_CODES.NESTING_DEPTH_LIMIT,
        `ACL parse limit exceeded: nesting depth is greater than ${this.limits.maxNestingDepth}`,
        token?.span ?? this.eofSpan()
      );
    }

    this.blockDepth++;
    try {
      return this.parseBlockUnchecked();
    } finally {
      this.blockDepth--;
    }
  }

  parseBlockUnchecked() {
    const name = this.current().value;
    this.advance();

    const labels = [];
    while (this.current()?.type === 'String') {
      this.ensureCollectionCapacity(labels.length);
      labels.push(this.current().value);
      this.advance();
    }

    if (this.current()?.type === 'LeftBrace') {
      this.advance();
      const { blocks, attributes } = this.parseBlockBody();
      return { name, labels, blocks, attributes };
    }

    // Implicit block without braces
    this.skipNewlines();
    const attributes = new Map();
    const blocks = [];
    let itemCount = 0;

    while (this.current()?.type === 'Ident') {
      const attrName = this.current().value;
      const itemToken = this.current();
      this.advance();

      if (this.current()?.type === 'Equal' || this.current()?.type === 'Colon') {
        this.ensureCollectionCapacity(itemCount, itemToken);
        this.advance();
        const value = this.parseValue();
        attributes.set(attrName, value);
        itemCount++;
      } else if (this.current()?.type === 'LeftBrace') {
        this.ensureCollectionCapacity(itemCount, itemToken);
        const nested = this.parseBlock();
        blocks.push(nested);
        itemCount++;
      } else if (this.current()?.type === 'String') {
        const label = this.current().value;
        this.advance();
        if (this.current()?.type === 'LeftBrace') {
          this.ensureCollectionCapacity(itemCount, itemToken);
          this.advance();
          const { blocks: nestedBlocks, attributes: nestedAttrs } = this.parseBlockBody();
          const nested = { name: attrName, labels: [label], blocks: nestedBlocks, attributes: nestedAttrs };
          blocks.push(nested);
          itemCount++;
        }
      }
      this.skipNewlines();
    }

    return { name, labels, blocks, attributes };
  }

  parseBlockBody() {
    const attributes = new Map();
    const blocks = [];
    let itemCount = 0;
    this.skipNewlines();

    while (this.current()?.type !== 'RightBrace' && this.current()?.type !== 'Eof') {
      this.skipNewlines();
      if (this.current()?.type === 'RightBrace') break;

      if (this.current()?.type === 'Ident') {
        const name = this.current().value;
        const itemToken = this.current();
        this.advance();

        if (this.current()?.type === 'Equal' || this.current()?.type === 'Colon') {
          this.ensureCollectionCapacity(itemCount, itemToken);
          this.advance();
          const value = this.parseValue();
          attributes.set(name, value);
          itemCount++;
        } else if (this.current()?.type === 'String') {
          const label = this.current().value;
          this.advance();
          if (this.current()?.type === 'LeftBrace') {
            this.ensureCollectionCapacity(itemCount, itemToken);
            this.advance();
            const { blocks: nestedBlocks, attributes: nestedAttrs } = this.parseBlockBody();
            const nested = { name, labels: [label], blocks: nestedBlocks, attributes: nestedAttrs };
            blocks.push(nested);
            itemCount++;
          }
        } else if (this.current()?.type === 'LeftBrace') {
          this.ensureCollectionCapacity(itemCount, itemToken);
          this.advance();
          const { blocks: nestedBlocks, attributes: nestedAttrs } = this.parseBlockBody();
          const nested = { name, labels: [], blocks: nestedBlocks, attributes: nestedAttrs };
          blocks.push(nested);
          itemCount++;
        }
      }
      this.skipNewlines();
    }

    if (this.current()?.type === 'RightBrace') {
      this.advance();
    }

    return { blocks, attributes };
  }

  parseValue() {
    const token = this.current();
    if (!token) {
      throw new ParseError(
        DIAGNOSTIC_CODES.UNEXPECTED_EOF,
        'Unexpected end of input',
        this.eofSpan()
      );
    }

    switch (token.type) {
      case 'String':
        this.advance();
        return { kind: 'String', value: token.value };
      case 'Number':
        this.advance();
        return { kind: 'Number', value: token.value };
      case 'True':
        this.advance();
        return { kind: 'Bool', value: true };
      case 'False':
        this.advance();
        return { kind: 'Bool', value: false };
      case 'Null':
        this.advance();
        return { kind: 'Null' };
      case 'LeftBracket':
        return this.parseList();
      case 'LeftBrace':
        return this.parseObject();
      case 'Ident': {
        const name = token.value;
        this.advance();
        this.skipNewlines();
        if (this.current()?.type === 'LeftParen') {
          this.advance();
          const args = this.parseCallArgs();
          return { kind: 'Call', name, args };
        }
        return { kind: 'String', value: name };
      }
      case 'Eof':
        throw new ParseError(
          DIAGNOSTIC_CODES.UNEXPECTED_EOF,
          'Unexpected end of input',
          token.span
        );
      default:
        throw new ParseError(
          DIAGNOSTIC_CODES.UNEXPECTED_TOKEN,
          `Unexpected token in value position: ${token.type}`,
          token.span
        );
    }
  }

  parseObject() {
    this.advance(); // consume '{'
    const pairs = new Map();
    let itemCount = 0;

    this.skipNewlines();
    while (this.current()?.type !== 'RightBrace' && this.current()?.type !== 'Eof') {
      if (this.current()?.type === 'Ident') {
        const key = this.current().value;
        const itemToken = this.current();
        this.advance();

        if (this.current()?.type === 'Equal' || this.current()?.type === 'Colon') {
          this.ensureCollectionCapacity(itemCount, itemToken);
          this.advance();
          const value = this.parseValue();
          pairs.set(key, value);
          itemCount += 1;
        }
      }
      this.skipNewlines();
      if (this.current()?.type === 'Comma') {
        this.advance();
        this.skipNewlines();
      }
    }

    if (this.current()?.type === 'RightBrace') {
      this.advance();
    }

    return { kind: 'Object', pairs: Array.from(pairs.entries()) };
  }

  parseList() {
    const items = [];
    this.advance();

    this.skipNewlines();
    while (this.current()?.type !== 'RightBracket' && this.current()?.type !== 'Eof') {
      this.ensureCollectionCapacity(items.length);
      const value = this.parseValue();
      items.push(value);
      this.skipNewlines();

      if (this.current()?.type === 'Comma') {
        this.advance();
        this.skipNewlines();
      }
    }

    if (this.current()?.type === 'RightBracket') {
      this.advance();
    }

    return { kind: 'List', items };
  }

  parseCallArgs() {
    const args = [];
    this.skipNewlines();

    if (this.current()?.type === 'RightParen') {
      this.advance();
      return args;
    }

    while (true) {
      this.ensureCollectionCapacity(args.length);
      const value = this.parseValue();
      args.push(value);
      this.skipNewlines();

      if (this.current()?.type === 'Comma') {
        this.advance();
        this.skipNewlines();
      } else if (this.current()?.type === 'RightParen') {
        this.advance();
        break;
      } else {
        const token = this.current();
        if (!token || token.type === 'Eof') {
          throw new ParseError(
            DIAGNOSTIC_CODES.UNEXPECTED_EOF,
            'Unexpected end of input in function call',
            token?.span ?? this.eofSpan()
          );
        }
        throw new ParseError(
          DIAGNOSTIC_CODES.EXPECTED_TOKEN,
          `Expected ',' or ')', found ${token.type}`,
          token.span
        );
      }
    }

    return args;
  }
}

function parse(input, limits) {
  const parser = new Parser([]);
  return parser.parse(input, limits);
}

function collectDiagnostics(input, limits) {
  const parser = new Parser([]);
  return parser.collectDiagnostics(input, limits);
}

module.exports = {
  collectDiagnostics,
  DEFAULT_PARSE_LIMITS,
  normalizeParseLimits,
  Parser,
  parse,
};
