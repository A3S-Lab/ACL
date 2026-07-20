const DIAGNOSTIC_CODES = Object.freeze({
  DOCUMENT_BYTES_LIMIT: 'acl.limit.document_bytes',
  TOKEN_BYTES_LIMIT: 'acl.limit.token_bytes',
  NESTING_DEPTH_LIMIT: 'acl.limit.nesting_depth',
  COLLECTION_ITEMS_LIMIT: 'acl.limit.collection_items',
  UNEXPECTED_TOKEN: 'acl.parse.unexpected_token',
  EXPECTED_TOKEN: 'acl.parse.expected_token',
  UNEXPECTED_EOF: 'acl.parse.unexpected_eof',
});

const START_OF_DOCUMENT = Object.freeze({ line: 1, column: 1, offset: 0 });

function pointSpan(location = START_OF_DOCUMENT) {
  return {
    start: { ...location },
    end: { ...location },
  };
}

class ParseError extends Error {
  constructor(code, message, span) {
    super(message);
    this.name = 'ParseError';
    this.code = code;
    this.span = span;
    this.line = span.start.line;
    this.column = span.start.column;
  }
}

module.exports = {
  DIAGNOSTIC_CODES,
  ParseError,
  pointSpan,
  START_OF_DOCUMENT,
};
