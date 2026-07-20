# ACL - Agent Configuration Language

A lightweight, typed configuration language for agent configurations. ACL is designed for defining structured agent behaviors with blocks, attributes, and function calls.

## Features

- **ACL Block Syntax**: Labeled blocks, attributes, nested blocks, and function calls defined by the A3S ACL grammar
- **Typed Values**: Strings, numbers, booleans, lists, objects, null, and function calls
- **Function Calls**: Built-in support for `env()`, `concat()`, and custom functions
- **Bidirectional**: Parse ACL text to AST, generate AST back to text
- **Type-Stable Strings**: Canonical generation quotes empty, numeric-looking,
  keyword-like, and Unicode strings so parsing cannot change their value kind
- **Stable Canonical Digests**: Rust and Node.js expose byte-identical
  canonical UTF-8 plus lowercase, algorithm-prefixed SHA-256 digests
- **Structured Diagnostics**: Stable cross-SDK codes, complete source spans,
  UTF-8 byte offsets, and messages that never echo source token values
- **Bounded Multi-Diagnostics**: Deterministic line recovery, configurable
  diagnostic budgets, and explicit truncation without changing fail-fast parsing
- **Schema Admission**: Closed-by-default document shapes, recursive value
  rules, stable logical paths, and bounded cross-SDK validation reports
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
| Collected diagnostics | 100 |

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
        max_diagnostics: 20,
    },
)?;
```

```typescript
const doc = parse(input, {
  maxDocumentBytes: 64 * 1024,
  maxNestingDepth: 32,
  maxCollectionItems: 1_000,
  maxTokenBytes: 16 * 1024,
  maxDiagnostics: 20,
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

The `parse` APIs remain fail-fast. CLI and editor integrations can collect
multiple errors without constructing a partial AST:

```rust
use a3s_acl::{collect_diagnostics_with_limits, ParseLimits};

let report = collect_diagnostics_with_limits(
    "first = ]\nsecond = ]",
    ParseLimits {
        max_diagnostics: 20,
        ..ParseLimits::default()
    },
);
```

```typescript
import {collectDiagnostics} from '@a3s-lab/acl';

const report = collectDiagnostics('first = ]\nsecond = ]', {
  maxDiagnostics: 20,
});
```

After a syntax error, collection resumes at the next source line. Resource-limit
diagnostics remain fatal and appear at most once. The collector stores no more
than `max_diagnostics` / `maxDiagnostics` errors and sets `truncated` only after
observing an additional error beyond that budget. A zero budget therefore
returns no diagnostics and sets `truncated` when the input is invalid.

### Schema admission

Parse untrusted input with explicit limits, then validate the resulting
document before activation:

```rust
use a3s_acl::{
    parse_with_limits, validate_document_with_limits, AttributeSchema,
    ParseLimits, Schema, ValueSchema,
};

let schema = Schema::new().attribute(
    "version",
    AttributeSchema::required(ValueSchema::number()),
);
let limits = ParseLimits {
    max_diagnostics: 20,
    ..ParseLimits::default()
};
let document = parse_with_limits("version = 1", limits)?;
let report = validate_document_with_limits(&document, &schema, limits);
assert!(report.is_empty());
```

```typescript
import {parse, validateDocument} from '@a3s-lab/acl';

const schema = {
  attributes: {
    version: {required: true, value: {kind: 'Number'}},
  },
};
const limits = {maxDiagnostics: 20};
const document = parse('version = 1', limits);
const report = validateDocument(document, schema, limits);
```

Schemas are closed by default. They can declare required or optional
attributes, nested block and label cardinalities, whether matching block
occurrences are semantically unordered, and recursive `Any`, `String`,
`Number`, `Bool`, `Null`, `List`, `Object`, `Call`, and `OneOf` value rules.
Unknown attributes, blocks, and object fields require an explicit allow flag.

Schema diagnostics use stable `acl.schema.*` codes and logical paths such as
`$.blocks.provider[0].attributes.api_key`. Messages and paths never include
attribute values, call arguments, or block labels. Reports use the same
diagnostic budget as parsing and set `truncated` only after observing an
additional schema error beyond that budget.

Schema validation checks document shape, not host-specific semantics such as
numeric ranges, secret resolution, or provider credentials. Those checks
remain the responsibility of the admitting component.

### Generate

```rust
let output = generate(doc: &Document) -> String
```

```typescript
const output = generate(doc: Document): string
```

The generator emits native ACL syntax. Block labels remain in block headers;
there is no label-as-attribute compatibility output.

### Canonical bytes and digest

Use the canonical APIs when ACL bytes are signed, stored, or compared across
SDKs:

```rust
use a3s_acl::{canonical_bytes, canonical_digest, parse};

let document = parse("limits { memory = 128000000 }")?;
let bytes = canonical_bytes(&document)?;
let digest = canonical_digest(&document)?;
assert!(digest.starts_with("sha256:"));
```

```typescript
import {canonicalBytes, canonicalDigest, parse} from '@a3s-lab/acl';

const document = parse('limits { memory = 128000000 }');
const bytes = canonicalBytes(document);
const digest = canonicalDigest(document);
```

Canonical bytes use the default ACL generator, UTF-8 without a byte-order
mark, LF line endings, and exactly one final LF. Attribute maps and object
pairs are semantically unordered: their portable ASCII identifiers are sorted
by ascending byte value, and duplicate object keys use the last value.
Document-level attributes remain assignments for every value kind, while an
unlabeled block with one differently named attribute retains its block braces;
canonicalization never changes one shape into the other.
Document and nested-block order, block-label order, list items, and function
arguments remain ordered and therefore affect the digest.

After schema admission, use the schema-aware APIs to normalize repeatable block
types that the trusted schema marks as unordered:

```rust
use a3s_acl::{canonical_digest_with_schema, BlockSchema, Schema};

let schema = Schema::new().block(
    "provider",
    BlockSchema::new(Schema::new()).unordered(true),
);
let digest = canonical_digest_with_schema(&document, &schema)?;
```

```typescript
import {canonicalDigestWithSchema} from '@a3s-lab/acl';

const schema = {
  blocks: {
    provider: {unordered: true},
  },
};
const digest = canonicalDigestWithSchema(document, schema);
```

Normalization is recursive. For each body, only occurrences of the same
declared block name whose rule sets `unordered` are sorted by canonical UTF-8
bytes. Their existing positions are retained, so other block types and unknown
blocks remain ordered. Schema-aware canonicalization does not perform
admission; validate the document first.

Finite numbers use the ECMAScript shortest round-tripping representation in
both SDKs, including `0` for negative zero and stable exponent boundaries.
Comments and source whitespace are discarded by parsing, while string and
label Unicode scalar sequences are preserved without NFC/NFD normalization.
Programmatic non-finite numbers, non-scalar JavaScript strings, and
non-portable identifiers fail with redacted `CanonicalError` values. Digests
are lowercase
`sha256:<64 hexadecimal characters>` strings over the exact canonical bytes.
The shared cases under `fixtures/canonical/digest-cases.json` and
`fixtures/canonical/schema-block-order-cases.json` are the cross-language
compatibility oracle.

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
