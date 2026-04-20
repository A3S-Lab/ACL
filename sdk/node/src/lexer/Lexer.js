/**
 * ACL Lexer - Tokenizer
 */

class Lexer {
  constructor(input) {
    this.input = input;
    this.pos = 0;
    this.line = 1;
    this.column = 1;
    this.offset = 0;
  }

  current() {
    return this.input[this.pos];
  }

  advance() {
    if (this.pos < this.input.length) {
      const c = this.input[this.pos];
      this.pos++;
      this.offset++;
      if (c === '\n') {
        this.line++;
        this.column = 1;
      } else {
        this.column++;
      }
    }
  }

  startLocation() {
    return { line: this.line, column: this.column, offset: this.offset };
  }

  makeToken(type, start, end) {
    return { type, span: { start, end } };
  }

  readStringDouble() {
    let result = '';
    while (this.current() !== undefined) {
      const c = this.current();
      if (c === '"') {
        this.advance();
        break;
      }
      if (c === '\\') {
        this.advance();
        const escaped = this.current();
        if (escaped) {
          switch (escaped) {
            case 'n': result += '\n'; break;
            case 'r': result += '\r'; break;
            case 't': result += '\t'; break;
            case '\\': result += '\\'; break;
            case '"': result += '"'; break;
            default: result += '\\' + escaped;
          }
          this.advance();
        }
      } else if (c === '\n') {
        break;
      } else {
        result += c;
        this.advance();
      }
    }
    return result;
  }

  readStringSingle() {
    let result = '';
    while (this.current() !== undefined) {
      const c = this.current();
      if (c === "'") {
        this.advance();
        break;
      }
      if (c === '\\') {
        this.advance();
        const escaped = this.current();
        if (escaped) {
          switch (escaped) {
            case "'": result += "'"; break;
            default: result += '\\' + escaped;
          }
          this.advance();
        }
      } else if (c === '\n') {
        break;
      } else {
        result += c;
        this.advance();
      }
    }
    return result;
  }

  readNumber() {
    const start = this.pos;
    let hasDot = false;
    let hasE = false;

    if (this.current() === '-') {
      this.advance();
    }

    while (this.current() !== undefined) {
      const c = this.current();
      if (c >= '0' && c <= '9') {
        this.advance();
      } else if (c === '.' && !hasDot && !hasE) {
        hasDot = true;
        this.advance();
      } else if ((c === 'e' || c === 'E') && !hasE) {
        hasE = true;
        this.advance();
        if (this.current() === '+' || this.current() === '-') {
          this.advance();
        }
      } else {
        break;
      }
    }

    const numStr = this.input.slice(start, this.pos);
    const value = parseFloat(numStr);
    return { value: isNaN(value) ? 0 : value };
  }

  readIdent() {
    const start = this.pos;

    while (this.current() !== undefined) {
      const c = this.current();
      if ((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || c === '_' || (c >= '0' && c <= '9')) {
        this.advance();
      } else {
        break;
      }
    }

    const name = this.input.slice(start, this.pos);
    return { name };
  }

  tokenize() {
    const tokens = [];

    while (this.current() !== undefined) {
      const start = this.startLocation();
      const c = this.current();

      switch (c) {
        case '{':
          this.advance();
          tokens.push(this.makeToken('LeftBrace', start, this.startLocation()));
          break;
        case '}':
          this.advance();
          tokens.push(this.makeToken('RightBrace', start, this.startLocation()));
          break;
        case '[':
          this.advance();
          tokens.push(this.makeToken('LeftBracket', start, this.startLocation()));
          break;
        case ']':
          this.advance();
          tokens.push(this.makeToken('RightBracket', start, this.startLocation()));
          break;
        case '(':
          this.advance();
          tokens.push(this.makeToken('LeftParen', start, this.startLocation()));
          break;
        case ')':
          this.advance();
          tokens.push(this.makeToken('RightParen', start, this.startLocation()));
          break;
        case '=':
          this.advance();
          tokens.push(this.makeToken('Equal', start, this.startLocation()));
          break;
        case '+':
          this.advance();
          if (this.current() === '=') {
            this.advance();
            tokens.push(this.makeToken('PlusEqual', start, this.startLocation()));
          }
          break;
        case ':':
          this.advance();
          tokens.push(this.makeToken('Colon', start, this.startLocation()));
          break;
        case ',':
          this.advance();
          tokens.push(this.makeToken('Comma', start, this.startLocation()));
          break;
        case '"':
          this.advance();
          const s = this.readStringDouble();
          tokens.push({ type: 'String', value: s, span: { start, end: this.startLocation() } });
          break;
        case "'":
          this.advance();
          const s2 = this.readStringSingle();
          tokens.push({ type: 'String', value: s2, span: { start, end: this.startLocation() } });
          break;
        case '#':
        case '/':
          this.advance();
          while (this.current() !== undefined && this.current() !== '\n') {
            this.advance();
          }
          tokens.push(this.makeToken('Comment', start, this.startLocation()));
          break;
        case '\n':
        case '\r':
          this.advance();
          tokens.push(this.makeToken('Newline', start, this.startLocation()));
          break;
        case ' ':
        case '\t':
          this.advance();
          break;
        default:
          if ((c >= '0' && c <= '9') || c === '-' || c === '.') {
            const { value } = this.readNumber();
            tokens.push({ type: 'Number', value, span: { start, end: this.startLocation() } });
          } else if ((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || c === '_') {
            const { name } = this.readIdent();
            let type = 'Ident';
            if (name === 'true') type = 'True';
            else if (name === 'false') type = 'False';
            else if (name === 'null') type = 'Null';
            tokens.push({ type, value: name, span: { start, end: this.startLocation() } });
          } else {
            this.advance();
          }
      }
    }

    tokens.push({ type: 'Eof', span: { start: this.startLocation(), end: this.startLocation() } });
    return tokens;
  }
}

module.exports = { Lexer };
