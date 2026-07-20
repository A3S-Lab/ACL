# ACL - Agent Configuration Language

A lightweight, typed configuration language for agent configurations. ACL is designed for defining structured agent behaviors with blocks, attributes, and function calls.

## Features

- **ACL Block Syntax**: Labeled blocks, attributes, nested blocks, and function calls defined by the A3S ACL grammar
- **Typed Values**: Strings, numbers, booleans, lists, objects, null, and function calls
- **Function Calls**: Built-in support for `env()`, `concat()`, and custom functions
- **Bidirectional**: Parse ACL text to AST, generate AST back to text
- **Type-Stable Strings**: Canonical generation quotes empty, numeric-looking,
  keyword-like, and Unicode strings so parsing cannot change their value kind
- **Structured Diagnostics**: Stable cross-SDK codes, complete source spans,
  UTF-8 byte offsets, and messages that never echo source token values
- **Multi-platform SDK**: Rust crate and Node.js/TypeScript SDK

## Syntax

```acl
providers "openai" {
    api_key = env("OPENAI_API_KEY")
    base_url = "https://api.openai.com/v1"
}

default_model = "gpt-4"

settings {
    temperature = 0.7
    max_tokens = 2000
}

nested "label" {
    deeply {
        value = "supported"
    }
}
```

## Installation

### Rust

```toml
[dependencies]
a3s-acl = "0.2.2"
```

```rust
use a3s_acl::{parse, generate, string, number, boolean, BlockBuilder, DocumentBuilder};

let doc = parse(r#"
    name = "test"
    count = 42
"#)?;

let output = generate(&doc);
```

### Node.js / TypeScript

```bash
npm install @a3s-lab/acl
```

```typescript
import { parse, generate, string, number, boolean, BlockBuilder } from '@a3s-lab/acl';

const doc = parse(`
    providers "openai" {
        api_key = env("OPENAI_API_KEY")
    }
`);

const output = generate(doc);
```

## API

### Parse

```rust
let doc = parse(input: &str) -> Result<Document, ParseError>
```

```typescript
const doc = parse(input: string): Document
```

The high-level parsers apply resource limits before recursively parsing
untrusted input. Defaults are identical in Rust and Node.js:

| Limit | Default |
| --- | ---: |
| UTF-8 document size | 1 MiB |
| Structural nesting depth | 64 |
| Items in one document or collection | 10,000 |
| UTF-8 source token size | 256 KiB |

Use explicit limits at API admission boundaries:

```rust
use a3s_acl::{parse_with_limits, ParseLimits};

let doc = parse_with_limits(
    input,
    ParseLimits {
        max_document_bytes: 64 * 1024,
        max_nesting_depth: 32,
        max_collection_items: 1_000,
        max_token_bytes: 16 * 1024,
    },
)?;
```

```typescript
const doc = parse(input, {
  maxDocumentBytes: 64 * 1024,
  maxNestingDepth: 32,
  maxCollectionItems: 1_000,
  maxTokenBytes: 16 * 1024,
});
```

Document and token sizes count UTF-8 bytes. Nesting includes blocks, lists,
objects, and function calls. Collection limits apply independently to the
document, each block body and label list, each list or object, and each
function argument list. Direct lexer use is an advanced API and is not a
substitute for the bounded high-level parser.

### Parse diagnostics

Rust and Node.js return the same stable diagnostic codes and source spans:

| Code | Meaning |
| --- | --- |
| `acl.limit.document_bytes` | Document byte limit exceeded |
| `acl.limit.token_bytes` | Source token byte limit exceeded |
| `acl.limit.nesting_depth` | Structural nesting limit exceeded |
| `acl.limit.collection_items` | Collection item limit exceeded |
| `acl.parse.unexpected_token` | Token is not valid at this grammar position |
| `acl.parse.expected_token` | A required delimiter or token kind is missing |
| `acl.parse.unexpected_eof` | Input ended before the current construct |

Locations use one-based lines and columns. Span offsets are zero-based UTF-8
byte offsets, so Rust and Node.js agree even when Unicode precedes an error.
The compatibility `line` and `column` fields equal `span.start.line` and
`span.start.column`.

```rust
use a3s_acl::{parse, DiagnosticCode};

let error = parse(r#""private-value""#).unwrap_err();
assert_eq!(error.code, DiagnosticCode::UnexpectedToken);
assert_eq!(error.code.as_str(), "acl.parse.unexpected_token");
assert_eq!(error.span.start.offset, 0);
```

```typescript
import { parse, ParseError } from '@a3s-lab/acl';

try {
  parse('"private-value"');
} catch (error) {
  if (error instanceof ParseError) {
    console.error(error.code, error.span);
  }
}
```

Diagnostics identify token kinds but never include token values or source
snippets. Callers should preserve that boundary and must not attach the
untrusted ACL document to API errors or logs.

### Generate

```rust
let output = generate(doc: &Document) -> String
```

```typescript
const output = generate(doc: Document): string
```

### Value Constructors

| Function | Rust | TypeScript |
|----------|------|------------|
| String | `string("x")` | `string("x")` |
| Number | `number(42.0)` | `number(42)` |
| Boolean | `boolean(true)` | `boolean(true)` |
| Null | `null_value()` | `nullValue()` |
| List | `list(vec![...])` | `list([...])` |
| Function Call | `call("env", vec![...])` | `call("env", [...])` |

### Builders

```rust
let block = BlockBuilder::new("config")
    .label("primary")
    .attr("name", string("test"))
    .attr("count", number(42))
    .nested_block(nested_block)
    .build();

let doc = DocumentBuilder::new()
    .block(block)
    .build();
```

```typescript
const block = new BlockBuilder('config')
    .label('primary')
    .attr('name', string('test'))
    .attr('count', number(42))
    .nestedBlock(nestedBlock)
    .build();

const doc = new DocumentBuilder()
    .block(block)
    .build();
```

## Value Types

| Kind | Description |
|------|-------------|
| `String` | Quoted text: `"hello"` |
| `Number` | Integer or float: `42`, `3.14` |
| `Bool` | Boolean: `true`, `false` |
| `Null` | Null value: `null` |
| `List` | Ordered collection: `[1, 2, 3]` |
| `Object` | Key-value pairs: `{key = value}` |
| `Call` | Function invocation: `env("VAR")` |

Canonical generation always quotes `String` values. For example, `""`, `"42"`,
`"1.88"`, `"true"`, and `"null"` remain strings after a parse/generate/parse
round trip. Rust and Node validate this rule against the shared fixture under
`fixtures/canonical/`.

## License

MIT
