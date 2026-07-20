/**
 * ACL SDK Tests
 */

const fs = require('node:fs');
const path = require('node:path');
const acl = require('./index.js');
const {
  CANONICAL_DIGEST_ALGORITHM,
  CanonicalError,
  ParseError,
  DEFAULT_PARSE_LIMITS,
  SCHEMA_DIAGNOSTIC_CODES,
  canonicalBytes,
  canonicalDigest,
  collectDiagnostics,
  validateDocument,
  parse,
  generate,
  string,
  number,
  boolean,
  call,
  BlockBuilder,
  DocumentBuilder,
} = acl;

// Helper to convert Map to Object for display
function mapToObj(m) {
  if (!m || typeof m !== 'object' || m.constructor !== Map) return m;
  return Object.fromEntries([...m].map(([k, v]) => [k, mapToObj(v)]));
}

function docToObj(doc) {
  return {
    blocks: doc.blocks.map(b => ({
      name: b.name,
      labels: b.labels,
      blocks: b.blocks.map(b2 => ({
        name: b2.name,
        labels: b2.labels,
        blocks: [],
        attributes: mapToObj(b2.attributes)
      })),
      attributes: mapToObj(b.attributes)
    }))
  };
}

// Assertion helper
function assert(condition, message) {
  if (!condition) {
    throw new Error('Assertion failed: ' + message);
  }
  console.log('  PASS: ' + message);
}

function limitFixture(name) {
  return fs.readFileSync(path.join(__dirname, '../../../fixtures/limits', name), 'utf8');
}

function expectLimitError(input, limits, expectedMessage) {
  try {
    parse(input, limits);
    throw new Error('Expected parsing to fail');
  } catch (error) {
    assert(error.message === expectedMessage, expectedMessage);
    return error;
  }
}

function diagnosticFixture() {
  return JSON.parse(
    fs.readFileSync(
      path.join(__dirname, '../../../fixtures/diagnostics/cases.json'),
      'utf8'
    )
  );
}

function multiDiagnosticFixture() {
  return JSON.parse(
    fs.readFileSync(
      path.join(__dirname, '../../../fixtures/diagnostics/multi-cases.json'),
      'utf8'
    )
  );
}

function schemaAdmissionFixture() {
  return JSON.parse(
    fs.readFileSync(
      path.join(__dirname, '../../../fixtures/schema/admission-cases.json'),
      'utf8'
    )
  );
}

function canonicalDigestFixture() {
  return JSON.parse(
    fs.readFileSync(
      path.join(__dirname, '../../../fixtures/canonical/digest-cases.json'),
      'utf8'
    )
  );
}

console.log('=== Test Canonical Digests ===');
assert(CANONICAL_DIGEST_ALGORITHM === 'sha256', 'canonical digest algorithm should be stable');
for (const testCase of canonicalDigestFixture()) {
  for (const input of [testCase.input, testCase.equivalentInput]) {
    const document = parse(input);
    assert(
      Buffer.from(canonicalBytes(document)).toString('utf8') === testCase.canonical,
      `${testCase.name} should have stable canonical bytes`
    );
    assert(
      canonicalDigest(document) === testCase.digest,
      `${testCase.name} should have a stable canonical digest`
    );
  }
}

const firstListDigest = canonicalDigest(parse('value = [1, 2]'));
const secondListDigest = canonicalDigest(parse('value = [2, 1]'));
assert(firstListDigest !== secondListDigest, 'canonical digests should preserve list order');
assert(
  canonicalDigest(parse('value = "é"')) !== canonicalDigest(parse('value = "e\u0301"')),
  'canonical digests should preserve Unicode scalar sequences without normalization'
);

try {
  canonicalDigest(
    new DocumentBuilder()
      .block(new BlockBuilder('value').attr('value', number(Number.POSITIVE_INFINITY)).build())
      .build()
  );
  throw new Error('Expected non-finite canonical number to fail');
} catch (error) {
  assert(
    error instanceof CanonicalError && error.code === 'acl.canonical.non_finite_number',
    'canonical digests should reject non-finite numbers with a stable error'
  );
}

try {
  canonicalDigest(
    new DocumentBuilder()
      .block(new BlockBuilder('value').attr('value', string('\uD800')).build())
      .build()
  );
  throw new Error('Expected non-scalar canonical string to fail');
} catch (error) {
  assert(
    error instanceof CanonicalError && error.code === 'acl.canonical.invalid_unicode',
    'canonical digests should reject non-scalar JavaScript strings'
  );
}

try {
  canonicalDigest(
    new DocumentBuilder()
      .block(new BlockBuilder('配置').attr('private', string('private-value')).build())
      .build()
  );
  throw new Error('Expected non-portable canonical identifier to fail');
} catch (error) {
  assert(
    error instanceof CanonicalError
      && error.code === 'acl.canonical.unsupported_identifier'
      && !error.message.includes('private-value'),
    'canonical identifier errors should be stable and redacted'
  );
}

console.log('=== Test Structured Diagnostics ===');
for (const testCase of diagnosticFixture()) {
  const limits = {
    maxDocumentBytes: testCase.limits.maxDocumentBytes,
    maxNestingDepth: testCase.limits.maxNestingDepth,
    maxCollectionItems: testCase.limits.maxCollectionItems,
    maxTokenBytes: testCase.limits.maxTokenBytes,
  };
  for (const key of Object.keys(limits)) {
    if (limits[key] === undefined) delete limits[key];
  }

  try {
    parse(testCase.input, limits);
    throw new Error(`${testCase.name} should fail`);
  } catch (error) {
    assert(error instanceof ParseError, `${testCase.name} should return ParseError`);
    assert(error.code === testCase.expected.code, `${testCase.name} should have a stable code`);
    assert(error.message === testCase.expected.message, `${testCase.name} should have a stable message`);
    assert(
      JSON.stringify(error.span) === JSON.stringify(testCase.expected.span),
      `${testCase.name} should have a byte-accurate span`
    );
    assert(
      error.line === error.span.start.line && error.column === error.span.start.column,
      `${testCase.name} should preserve line and column compatibility`
    );
    assert(!error.message.includes('TOP_SECRET'), `${testCase.name} should redact source values`);
  }
}

console.log('=== Test Bounded Multi-Diagnostics ===');
for (const testCase of multiDiagnosticFixture()) {
  const report = collectDiagnostics(testCase.input, testCase.limits);
  assert(
    report.truncated === testCase.expected.truncated,
    `${testCase.name} should have a stable truncation flag`
  );
  assert(
    report.diagnostics.length === testCase.expected.diagnostics.length,
    `${testCase.name} should have a bounded diagnostic count`
  );
  for (const [index, error] of report.diagnostics.entries()) {
    const expected = testCase.expected.diagnostics[index];
    assert(error instanceof ParseError, `${testCase.name} diagnostic ${index} should be ParseError`);
    assert(error.code === expected.code, `${testCase.name} diagnostic ${index} should have a stable code`);
    assert(error.message === expected.message, `${testCase.name} diagnostic ${index} should have a stable message`);
    assert(
      JSON.stringify(error.span) === JSON.stringify(expected.span),
      `${testCase.name} diagnostic ${index} should have a byte-accurate span`
    );
    assert(!error.message.includes('secret'), `${testCase.name} diagnostic ${index} should redact source values`);
  }
}

const adversarialReport = collectDiagnostics(
  Array.from({ length: 1_000 }, (_, index) => `invalid_${index} = ]`).join('\n'),
  { maxDiagnostics: 3 }
);
assert(adversarialReport.diagnostics.length === 3, 'adversarial diagnostics should stay bounded');
assert(adversarialReport.truncated, 'adversarial diagnostics should report truncation');

const exactBudgetReport = collectDiagnostics(
  'first = ]\nsecond = ]\nthird = ]',
  { maxDiagnostics: 3 }
);
assert(exactBudgetReport.diagnostics.length === 3, 'exact diagnostic budget should be retained');
assert(!exactBudgetReport.truncated, 'exact diagnostic budget should not report truncation');

console.log('=== Test Schema Admission ===');
const schemaFixture = schemaAdmissionFixture();
assert(
  SCHEMA_DIAGNOSTIC_CODES.VALUE_TYPE === 'acl.schema.value_type',
  'schema diagnostic codes should expose stable wire values'
);
for (const testCase of schemaFixture.cases) {
  const report = validateDocument(
    parse(testCase.input),
    schemaFixture.schema,
    testCase.limits
  );
  assert(
    report.truncated === testCase.expected.truncated,
    `${testCase.name} should have a stable schema truncation flag`
  );
  assert(
    report.diagnostics.length === testCase.expected.diagnostics.length,
    `${testCase.name} should have a stable schema diagnostic count`
  );
  for (const [index, diagnostic] of report.diagnostics.entries()) {
    const expected = testCase.expected.diagnostics[index];
    assert(diagnostic.code === expected.code, `${testCase.name} schema diagnostic ${index} should have a stable code`);
    assert(diagnostic.message === expected.message, `${testCase.name} schema diagnostic ${index} should have a stable message`);
    assert(diagnostic.path === expected.path, `${testCase.name} schema diagnostic ${index} should have a stable path`);
    assert(!diagnostic.message.includes('TOP_SECRET'), `${testCase.name} schema diagnostic ${index} should redact values`);
    assert(!diagnostic.path.includes('TOP_SECRET'), `${testCase.name} schema diagnostic ${index} should redact labels`);
  }
}

const exactSchemaBudget = validateDocument(
  parse('version = "TOP_SECRET"\nextra = "TOP_SECRET"'),
  schemaFixture.schema,
  { maxDiagnostics: 3 }
);
assert(exactSchemaBudget.diagnostics.length === 3, 'exact schema budget should be retained');
assert(!exactSchemaBudget.truncated, 'exact schema budget should not report truncation');

const adversarialSchemaReport = validateDocument(
  parse(Array.from(
    { length: 1_000 },
    (_, index) => `unknown_${index} = "TOP_SECRET"`
  ).join('\n')),
  schemaFixture.schema,
  { maxDiagnostics: 3 }
);
assert(adversarialSchemaReport.diagnostics.length === 3, 'adversarial schema diagnostics should stay bounded');
assert(adversarialSchemaReport.truncated, 'adversarial schema diagnostics should report truncation');

const openSchemaReport = validateDocument(
  parse('extension = "value"\ncustom "label" { nested = true }'),
  { allowUnknownAttributes: true, allowUnknownBlocks: true }
);
assert(openSchemaReport.diagnostics.length === 0, 'open schema flags should accept extension points');
assert(!openSchemaReport.truncated, 'valid open schema input should not report truncation');

const duplicateObjectReport = validateDocument(
  {
    blocks: [{
      name: 'payload',
      labels: [],
      blocks: [],
      attributes: new Map([[
        'payload',
        {
          kind: 'Object',
          pairs: [
            ['owner', { kind: 'String', value: 'first' }],
            ['owner', { kind: 'String', value: 'TOP_SECRET' }],
          ],
        },
      ]]),
    }],
  },
  {
    attributes: {
      payload: {
        required: true,
        value: {
          kind: 'Object',
          fields: {
            owner: { required: true, value: { kind: 'String' } },
          },
        },
      },
    },
  }
);
assert(
  duplicateObjectReport.diagnostics[0].code === SCHEMA_DIAGNOSTIC_CODES.DUPLICATE_OBJECT_FIELD,
  'programmatic duplicate object fields should be rejected'
);
assert(
  !duplicateObjectReport.diagnostics[0].message.includes('TOP_SECRET'),
  'duplicate object diagnostics should redact values'
);

const cyclicValue = { kind: 'List', items: [] };
cyclicValue.items.push(cyclicValue);
try {
  validateDocument(
    {
      blocks: [{
        name: 'payload',
        labels: [],
        blocks: [],
        attributes: new Map([['payload', cyclicValue]]),
      }],
    },
    { attributes: { payload: { value: { kind: 'Any' } } } }
  );
  throw new Error('Expected cyclic document to fail');
} catch (error) {
  assert(
    error instanceof TypeError && error.message.includes('cyclic Document'),
    'programmatic document cycles should fail before validation'
  );
}

try {
  validateDocument(parse(''), {
    blocks: {
      invalid: {
        occurrences: { min: 2, max: 1 },
      },
    },
  });
  throw new Error('Expected invalid schema cardinality to fail');
} catch (error) {
  assert(
    error instanceof TypeError && error.message.includes('occurrences'),
    'invalid schema cardinality should fail before validation'
  );
}

try {
  validateDocument(parse(''), {
    attributes: {
      invalid: {
        value: { kind: 'OneOf', variants: [] },
      },
    },
  });
  throw new Error('Expected empty schema union to fail');
} catch (error) {
  assert(
    error instanceof TypeError && error.message.includes('variants'),
    'empty schema unions should fail before validation'
  );
}

try {
  validateDocument(parse(''), { allowUnknownAttribute: true });
  throw new Error('Expected unknown schema field to fail');
} catch (error) {
  assert(
    error instanceof TypeError && error.message.includes('allowUnknownAttribute'),
    'unknown schema fields should fail before validation'
  );
}

const cyclicValueSchema = { kind: 'List' };
cyclicValueSchema.item = cyclicValueSchema;
try {
  validateDocument(parse(''), {
    attributes: {
      invalid: { value: cyclicValueSchema },
    },
  });
  throw new Error('Expected cyclic schema to fail');
} catch (error) {
  assert(
    error instanceof TypeError && error.message.includes('schema cycle'),
    'schema cycles should fail before validation'
  );
}

console.log('=== Test Bounded Parsing ===');
assert(Object.isFrozen(DEFAULT_PARSE_LIMITS), 'default parse limits should be immutable');
assert(
  DEFAULT_PARSE_LIMITS.maxDocumentBytes === 1024 * 1024
    && DEFAULT_PARSE_LIMITS.maxNestingDepth === 64
    && DEFAULT_PARSE_LIMITS.maxCollectionItems === 10_000
    && DEFAULT_PARSE_LIMITS.maxTokenBytes === 256 * 1024
    && DEFAULT_PARSE_LIMITS.maxDiagnostics === 100,
  'Rust and Node.js should publish the documented default limits'
);
const tokenLimitFixture = limitFixture('token.acl');
const nestingLimitFixture = limitFixture('nested.acl');
const collectionLimitFixture = limitFixture('collection.acl');

const documentLimit = Buffer.byteLength(tokenLimitFixture, 'utf8') - 1;
const documentError = expectLimitError(
  tokenLimitFixture,
  { maxDocumentBytes: documentLimit },
  `ACL parse limit exceeded: document is larger than ${documentLimit} bytes`
);
assert(documentError.line === 1 && documentError.column === 1, 'document limit should point to the start of input');
expectLimitError(
  'name = "智谱"',
  { maxDocumentBytes: Buffer.byteLength('name = "智谱"', 'utf8') - 1 },
  'ACL parse limit exceeded: document is larger than 14 bytes'
);

const tokenError = expectLimitError(
  tokenLimitFixture,
  { maxTokenBytes: 8 },
  'ACL parse limit exceeded: token is longer than 8 bytes'
);
assert(tokenError.line === 1 && tokenError.column === 8, 'token limit should point to the oversized token');

expectLimitError(
  'name = "智谱"',
  { maxTokenBytes: 7 },
  'ACL parse limit exceeded: token is longer than 7 bytes'
);

const nestingError = expectLimitError(
  nestingLimitFixture,
  { maxNestingDepth: 1 },
  'ACL parse limit exceeded: nesting depth is greater than 1'
);
assert(nestingError.line === 2 && nestingError.column === 9, 'nesting limit should point to the opening delimiter');

const collectionError = expectLimitError(
  collectionLimitFixture,
  { maxCollectionItems: 2 },
  'ACL parse limit exceeded: collection has more than 2 items'
);
assert(collectionError.line === 1, 'collection limit should point to the oversized collection');

for (const input of [
  'first = 1\nsecond = 2\nthird = 3',
  'root { first = 1 second = 2 third = 3 }',
  'root "first" "second" "third" { value = true }',
  'value = { first = 1 second = 2 third = 3 }',
  'value = concat(1, 2, 3)',
]) {
  expectLimitError(
    input,
    { maxCollectionItems: 2 },
    'ACL parse limit exceeded: collection has more than 2 items'
  );
}

parse(tokenLimitFixture);
parse(nestingLimitFixture);
parse(collectionLimitFixture);

try {
  parse('name = "test"', { maxDocumentBytes: Number.POSITIVE_INFINITY });
  throw new Error('Expected invalid parse limits to fail');
} catch (error) {
  assert(
    error instanceof TypeError && error.message.includes('maxDocumentBytes'),
    'invalid JavaScript limits should fail before parsing'
  );
}

try {
  collectDiagnostics('name = ]', { maxDiagnostics: -1 });
  throw new Error('Expected invalid diagnostic limit to fail');
} catch (error) {
  assert(
    error instanceof TypeError && error.message.includes('maxDiagnostics'),
    'invalid JavaScript diagnostic limits should fail before parsing'
  );
}

// Test basic parsing
console.log('=== Test Basic Parsing ===');
const doc1 = parse('name = "test"');
console.log('Parsed:', JSON.stringify(docToObj(doc1), null, 2));

// Test generate
console.log('\n=== Test Generate ===');
const output = generate(doc1);
console.log('Generated:\n' + output);

// Test function calls
console.log('\n=== Test Function Calls ===');
const doc2 = parse('api_key = env("API_KEY")');
console.log('Function call parsed:', JSON.stringify(docToObj(doc2), null, 2));
console.log('Generated:', generate(doc2));

// Test complex config
console.log('\n=== Test Complex Config ===');
const config = `
providers "openai" {
    api_key = env("OPENAI_API_KEY")
    base_url = "https://api.openai.com/v1"
}

default_model = "gpt-4"

settings {
    temperature = 0.7
    max_tokens = 2000
}
`;
const doc3 = parse(config);
console.log('Parsed complex config');
console.log('Blocks:', doc3.blocks.length);
console.log('Generated:\n' + generate(doc3));

// Test builder
console.log('\n=== Test Builder ===');
const block = new BlockBuilder('config')
    .attr('name', string('test'))
    .attr('count', number(42))
    .attr('enabled', boolean(true))
    .build();

const doc = new DocumentBuilder()
    .block(block)
    .build();

console.log('Built document:', JSON.stringify(docToObj(doc), null, 2));
console.log('Generated:\n' + generate(doc));

// Test nested blocks
console.log('\n=== Test Nested Blocks ===');
const nested = parse(`
providers "aws" {
    regions "us-east-1" {
        endpoint = "https://east.example.com"
    }
    regions "us-west-2" {
        endpoint = "https://west.example.com"
    }
}
`);
console.log('Nested parsed, blocks:', nested.blocks.length);
const nestedOutput = generate(nested);
assert(nestedOutput.includes('providers "aws"'), 'generate should preserve top-level ACL labels');
assert(nestedOutput.includes('regions "us-east-1"'), 'generate should preserve nested ACL labels');
assert(!Object.hasOwn(acl, 'generateHCL'), 'the obsolete generateHCL API should not be exported');
try {
  generate(nested, { labelsAsAttrs: true });
  throw new Error('Expected generator options to be rejected');
} catch (error) {
  assert(
    error instanceof TypeError && error.message === 'ACL generator options are not supported',
    'obsolete label-as-attribute options should fail explicitly'
  );
}
console.log('Generated:\n' + nestedOutput);

console.log('\n=== Test canonical string fixture ===');
const canonicalStringFixture = fs
    .readFileSync(path.join(__dirname, '../../../fixtures/canonical/string-values.acl'), 'utf8')
    .replace(/\r?\n$/, '');
const canonicalStringDocument = parse(canonicalStringFixture);
assert(
    generate(canonicalStringDocument) === canonicalStringFixture,
    'shared canonical string fixture should round trip byte-for-byte'
);
for (const [name, value] of canonicalStringDocument.blocks[0].attributes) {
    assert(value.kind === 'String', `${name} should remain a string`);
}

console.log('\n=== All tests passed! ===');
