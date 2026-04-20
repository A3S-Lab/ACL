# ACL - Agent Configuration Language

A lightweight, typed configuration language for agent configurations. ACL is designed for defining structured agent behaviors with blocks, attributes, and function calls.

## Features

- **HCL-like Syntax**: Familiar block-based structure with attributes and nested blocks
- **Typed Values**: Strings, numbers, booleans, lists, objects, null, and function calls
- **Function Calls**: Built-in support for `env()`, `concat()`, and custom functions
- **Bidirectional**: Parse ACL text to AST, generate AST back to text
- **Multi-platform SDK**: Rust crate and Node.js/TypeScript SDK

## Syntax

```hcl
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
a3s-acl = "0.1.0"
```

```rust
use acl::{parse, generate, string, number, boolean, BlockBuilder, DocumentBuilder};

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

## License

MIT
