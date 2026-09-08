# ACL - Agent Configuration Language（Agent 配置语言）

<p align="center">
  <strong>Language / 语言:</strong>
  <a href="README.md">English</a> ·
  <a href="README.zh-CN.md">中文</a>
</p>

用于代理配置的轻量级类型配置语言。 ACL 旨在通过块、属性和函数调用定义结构化代理行为。

## 功能

- **ACL 块语法**：A3S ACL 语法定义的标记块、属性、嵌套块和函数调用
- **类型化值**：字符串、数字、布尔值、列表、对象、null 和函数调用
- **函数调用**：内置支持 `env()`、`concat()` 和自定义函数
- **双向**：将 ACL 文本解析为 AST，将 AST 生成回文本
- **类型稳定的字符串**：规范生成引用空，数字外观，
  类似关键字和 Unicode 字符串，因此解析无法更改它们的值类型
- **稳定的规范摘要**：Rust 和 Node.js 公开字节相同
  规范的 UTF-8 加小写、算法前缀的 SHA-256 摘要
- **结构化诊断**：稳定的跨 SDK 代码、完整的源代码跨度、
  UTF-8 字节偏移量和从不回显源标记值的消息
- **有界多重诊断**：确定性线路恢复，可配置
  诊断预算和显式截断而不改变快速失败解析
- **模式准入**：默认关闭的文档形状，递归值
  规则、稳定的逻辑路径和有界的跨SDK验证报告
- **多平台 SDK**：Rust crate 和 Node.js/TypeScript SDK

## 语法

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

## 安装

### 铁锈

```toml
[dependencies]
a3s-acl = "0.3.0"
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

### 解析

```rust
let doc = parse(input: &str) -> Result<Document, ParseError>
```

```typescript
const doc = parse(input: string): Document
```

高级解析器在递归解析之前应用资源限制
不受信任的输入。 Rust 和 Node.js 中的默认值是相同的：

|限制|默认|
| ---| ---: |
| UTF-8 文档大小 | 1 MiB |
|结构嵌套深度| 64 | 64
|一个文档或集合中的项目 | 10,000 |
| UTF-8 源标记大小 | 256 KiB | 256 KiB
|收集的诊断信息 | 100 | 100

在 API 准入边界使用明确的限制：

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

文档和令牌大小按 UTF-8 字节计数。嵌套包括块、列表、
对象和函数调用。收集限额独立适用于
文档、每个块体和标签列表、每个列表或对象以及每个
函数参数列表。直接使用词法分析器是一种高级 API，而不是
替代有界高级解析器。

### 解析诊断

Rust 和 Node.js 返回相同的稳定诊断代码和源范围：

|代码|意义|
| --- | --- |
| `acl.limit.document_bytes` |超出文档字节限制 |
| `acl.limit.token_bytes` |超出源令牌字节限制 |
| `acl.limit.nesting_深度` |超出结构嵌套限制 |
| `acl.limit.collection_items` |超出收藏品限制 |
| `acl.parse.unexpected_token` |标记在此语法位置无效 |
| `acl.parse.expected_token` |缺少必需的分隔符或标记类型 |
| `acl.parse.unexpected_eof` |输入在当前构造之前结束 |

位置使用基于 1 的行和列。跨度偏移量是从零开始的 UTF-8
字节偏移量，因此即使 Unicode 在错误之前，Rust 和 Node.js 也是一致的。
兼容性"line"和"column"字段等于"span.start.line"并且
`span.start.column`。

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

诊断识别令牌类型，但从不包括令牌值或来源
片段。调用者应保留该边界并且不得附加
将不受信任的 ACL 文档记录到 API 错误或日志。

"parse" API 仍然是快速失败的。 CLI 和编辑器集成可以收集
未构建部分 AST 时出现多个错误：

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

出现语法错误后，收集将在下一个源行处恢复。资源限制
诊断仍然是致命的并且最多出现一次。收藏家不再储存
比 `max_diagnostics` / `maxDiagnostics` 错误并仅在之后设置 `truncated`
观察超出该预算的额外错误。因此预算为零
当输入无效时，不返回任何诊断信息并设置"截断"。

### 架构准入

使用明确的限制解析不受信任的输入，然后验证结果
激活前文档：

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

默认情况下模式是关闭的。他们可以声明必需或可选
属性、嵌套块和标签基数、是否匹配块
出现的次数在语义上是无序的，并且递归"Any"、"String"、
`Number`、`Bool`、`Null`、`List`、`Object`、`Call` 和 `OneOf` 值规则。
未知的属性、块和对象字段需要显式允许标志。

架构诊断使用稳定的"acl.schema.*"代码和逻辑路径，例如
`$.blocks.provider[0].attributes.api_key`。消息和路径从不包括
属性值、调用参数或块标签。报告使用相同的
诊断预算作为解析并仅在观察到后设置"截断"
超出该预算的额外架构错误。

模式验证检查文档形状，而不是特定于主机的语义，例如
数字范围、秘密解析或提供者凭据。那些支票
仍然是录取部分的责任。

### 产生

```rust
let output = generate(doc: &Document) -> String
```

```typescript
const output = generate(doc: Document): string
```

生成器发出本机 ACL 语法。块标签保留在块头中；
没有标签作为属性兼容性输出。

### 规范字节和摘要

在对 ACL 字节进行签名、存储或比较时使用规范 API
软件开发工具包：

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

规范字节使用默认的 ACL 生成器，没有字节顺序的 UTF-8
标记，LF 行结尾，以及最后一个 LF。属性映射和对象
对在语义上是无序的：它们的可移植 ASCII 标识符已排序
按升序字节值，重复的对象键使用最后一个值。
文档级属性仍然是每个值类型的分配，而
具有一个不同命名属性的未标记块保留其块大括号；
规范化永远不会将一种形状改变为另一种形状。
文档和嵌套块顺序、块标签顺序、列表项和函数
参数保持有序，因此会影响摘要。

模式接纳后，使用模式感知 API 规范化可重复块
受信任架构标记为无序的类型：

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

标准化是递归的。对于每个主体，仅出现相同的情况
声明的块名称，其规则集"unordered"按规范 UTF-8 排序
字节。他们现有的位置被保留，因此其他块类型和未知
块保持有序。模式感知规范化不执行
入场；首先验证文档。

有限数使用 ECMAScript 最短往返表示
两个 SDK，包括表示负零和稳定指数边界的"0"。
注释和源空白会被解析丢弃，而字符串和
保留标签 Unicode 标量序列，无需 NFC/NFD 标准化。
编程的非有限数字、非标量 JavaScript 字符串，以及
不可移植标识符失败并显示经过编辑的"CanonicalError"值。摘要
是小写的
精确规范字节上的"sha256:<64 个十六进制字符>"字符串。
`fixtures/canonical/digest-cases.json` 下的共享案例和
`fixtures/canonical/schema-block-order-cases.json` 是跨语言的
兼容性预言机。

### 值构造函数

|功能|铁锈|打字稿 |
|----------|------|------------|
|字符串| `字符串（"x"）` | `字符串（"x"）` |
|数量 | `数字（42.0）` | `数字（42）` |
|布尔 | `布尔值（真）` | `布尔值（真）` |
|空| `null_value()` | `nullValue()` |
|列表 | `列表（vec！[...]）` | `列表（[...]）` |
|函数调用| `call("env", vec![...])` | `call("env", [...])` |

### 建设者

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

## 值类型

|亲切 |描述 |
|------|-------------|
| `字符串` |引用文本：`"你好"` |
| `数量` |整数或浮点数：`42`、`3.14` |
| `布尔` |布尔值：`true`、`false` |
| `空` |空值：`null` |
| `列表` |有序集合：`[1, 2, 3]` |
| `对象` |键值对：`{key = value}` |
| `呼叫` |函数调用：`env("VAR")` |

规范生成始终引用"String"值。例如，`""`、`"42"`、
"1.88"、"true"和"null"在解析/生成/解析后仍保留字符串
往返。 Rust 和 Node 根据下面的共享夹具验证此规则
`固定装置/规范/`。

## 许可证

麻省理工学院