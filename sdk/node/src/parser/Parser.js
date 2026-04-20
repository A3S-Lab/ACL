/**
 * ACL Parser
 */

const { Lexer } = require('../lexer/Lexer.js');

class Parser {
  constructor(tokens) {
    this.tokens = tokens;
    this.pos = 0;
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

  parse(input) {
    const lexer = new Lexer(input);
    this.tokens = lexer.tokenize();
    this.pos = 0;
    return this.parseDocument();
  }

  parseDocument() {
    const blocks = [];
    this.skipNewlines();

    while (this.current()?.type !== 'Eof') {
      this.skipNewlines();
      if (this.current()?.type === 'Eof') break;

      if (this.current()?.type === 'Ident') {
        // Check if this is a bare attribute (name = value) or a block
        const next = this.peek();
        if (next?.type === 'Equal' || next?.type === 'Colon') {
          const block = this.parseBareAttribute();
          blocks.push(block);
        } else {
          const block = this.parseBlock();
          blocks.push(block);
        }
      } else {
        const err = this.current();
        throw { message: `Unexpected token: ${err?.type}`, line: err?.span.start.line ?? 0, column: err?.span.start.column ?? 0 };
      }
      this.skipNewlines();
    }

    return { blocks };
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
    const name = this.current().value;
    this.advance();

    const labels = [];
    while (this.current()?.type === 'String') {
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

    while (this.current()?.type === 'Ident') {
      const attrName = this.current().value;
      this.advance();

      if (this.current()?.type === 'Equal' || this.current()?.type === 'Colon') {
        this.advance();
        const value = this.parseValue();
        attributes.set(attrName, value);
      } else if (this.current()?.type === 'LeftBrace') {
        const nested = this.parseBlock();
        blocks.push(nested);
      } else if (this.current()?.type === 'String') {
        const label = this.current().value;
        this.advance();
        if (this.current()?.type === 'LeftBrace') {
          this.advance();
          const { blocks: nestedBlocks, attributes: nestedAttrs } = this.parseBlockBody();
          const nested = { name: attrName, labels: [label], blocks: nestedBlocks, attributes: nestedAttrs };
          blocks.push(nested);
        }
      }
      this.skipNewlines();
    }

    return { name, labels, blocks, attributes };
  }

  parseBlockBody() {
    const attributes = new Map();
    const blocks = [];
    this.skipNewlines();

    while (this.current()?.type !== 'RightBrace' && this.current()?.type !== 'Eof') {
      this.skipNewlines();
      if (this.current()?.type === 'RightBrace') break;

      if (this.current()?.type === 'Ident') {
        const name = this.current().value;
        this.advance();

        if (this.current()?.type === 'Equal' || this.current()?.type === 'Colon') {
          this.advance();
          const value = this.parseValue();
          attributes.set(name, value);
        } else if (this.current()?.type === 'String') {
          const label = this.current().value;
          this.advance();
          if (this.current()?.type === 'LeftBrace') {
            this.advance();
            const { blocks: nestedBlocks, attributes: nestedAttrs } = this.parseBlockBody();
            const nested = { name, labels: [label], blocks: nestedBlocks, attributes: nestedAttrs };
            blocks.push(nested);
          }
        } else if (this.current()?.type === 'LeftBrace') {
          this.advance();
          const { blocks: nestedBlocks, attributes: nestedAttrs } = this.parseBlockBody();
          const nested = { name, labels: [], blocks: nestedBlocks, attributes: nestedAttrs };
          blocks.push(nested);
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
    if (!token) throw { message: 'Unexpected end of input', line: 0, column: 0 };

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
      default:
        throw { message: `Unexpected token in value position: ${token.type}`, line: token.span.start.line, column: token.span.start.column };
    }
  }

  parseObject() {
    this.advance(); // consume '{'
    const pairs = [];

    this.skipNewlines();
    while (this.current()?.type !== 'RightBrace' && this.current()?.type !== 'Eof') {
      if (this.current()?.type === 'Ident') {
        const key = this.current().value;
        this.advance();

        if (this.current()?.type === 'Equal' || this.current()?.type === 'Colon') {
          this.advance();
          const value = this.parseValue();
          pairs.push([key, value]);
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

    return { kind: 'Object', pairs };
  }

  parseList() {
    const items = [];
    this.advance();

    this.skipNewlines();
    while (this.current()?.type !== 'RightBracket' && this.current()?.type !== 'Eof') {
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
        throw { message: `Expected ',' or ')', found: ${this.current()?.type}`, line: this.current()?.span.start.line ?? 0, column: this.current()?.span.start.column ?? 0 };
      }
    }

    return args;
  }
}

function parse(input) {
  const parser = new Parser([]);
  return parser.parse(input);
}

module.exports = { Parser, parse };
