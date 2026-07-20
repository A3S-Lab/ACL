/**
 * ACL SDK Tests
 */

const fs = require('node:fs');
const path = require('node:path');
const {
  ParseError,
  DEFAULT_PARSE_LIMITS,
  parse,
  generate,
  generateHCL,
  string,
  number,
  boolean,
  call,
  BlockBuilder,
  DocumentBuilder,
} = require('./index.js');

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

console.log('=== Test Bounded Parsing ===');
assert(Object.isFrozen(DEFAULT_PARSE_LIMITS), 'default parse limits should be immutable');
assert(
  DEFAULT_PARSE_LIMITS.maxDocumentBytes === 1024 * 1024
    && DEFAULT_PARSE_LIMITS.maxNestingDepth === 64
    && DEFAULT_PARSE_LIMITS.maxCollectionItems === 10_000
    && DEFAULT_PARSE_LIMITS.maxTokenBytes === 256 * 1024,
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
console.log('Generated:\n' + generate(nested));

// ============================================
// Tests for generateHCL (labelsAsAttrs mode)
// ============================================

console.log('\n=== Test generateHCL: default_model ===');
const hcl1 = new DocumentBuilder()
    .block(new BlockBuilder('default_model').attr('default_model', string('openai/gpt-4')).build())
    .build();
const hclOutput1 = generateHCL(hcl1);
assert(hclOutput1.includes('default_model = "openai/gpt-4"'), 'default_model should be bare attribute with quoted value');
console.log('Generated HCL:\n' + hclOutput1);

console.log('\n=== Test generateHCL: providers with labels ===');
const hcl2 = new DocumentBuilder()
    .block(
        new BlockBuilder('providers')
            .label('openai')
            .attr('base_url', string('https://api.openai.com/v1'))
            .build()
    )
    .build();
const hclOutput2 = generateHCL(hcl2);
assert(hclOutput2.includes('providers {'), 'providers should be block without label');
assert(hclOutput2.includes('name = "openai"'), 'label should be output as name attribute with quotes');
assert(!hclOutput2.includes('providers "openai"'), 'ACL-style label should not appear');
console.log('Generated HCL:\n' + hclOutput2);

console.log('\n=== Test generateHCL: models with labels ===');
const modelBlock = new BlockBuilder('models')
    .label('kimi-k2.5')
    .attr('api_key', string('sk-xxx'))
    .build();
const hcl3 = new DocumentBuilder()
    .block(
        new BlockBuilder('providers')
            .label('openai')
            .nestedBlock(modelBlock)
            .build()
    )
    .build();
const hclOutput3 = generateHCL(hcl3);
assert(hclOutput3.includes('models {'), 'models should be block without label');
assert(hclOutput3.includes('id = "kimi-k2.5"'), 'model label should be output as id attribute');
assert(!hclOutput3.includes('models "kimi-k2.5"'), 'ACL-style label should not appear');
console.log('Generated HCL:\n' + hclOutput3);

console.log('\n=== Test generateHCL: model with / and - in id ===');
const hcl4 = new DocumentBuilder()
    .block(
        new BlockBuilder('models')
            .label('openai/MiniMax-M2.7-highspeed')
            .attr('api_key', string('sk-xxx'))
            .build()
    )
    .build();
const hclOutput4 = generateHCL(hcl4);
assert(hclOutput4.includes('id = "openai/MiniMax-M2.7-highspeed"'), 'id with / and - should be quoted');
console.log('Generated HCL:\n' + hclOutput4);

console.log('\n=== Test generateHCL: complete agent config ===');
const defaultModelBlock = new BlockBuilder('default_model')
    .attr('default_model', string('openai/kimi-k2.5'))
    .build();
const openaiProvider = new BlockBuilder('providers')
    .label('openai')
    .attr('base_url', string('https://api.openai.com/v1'))
    .nestedBlock(
        new BlockBuilder('models')
            .label('kimi-k2.5')
            .attr('api_key', string('sk-test123'))
            .build()
    )
    .build();
const hcl5 = new DocumentBuilder()
    .block(defaultModelBlock)
    .block(openaiProvider)
    .build();
const hclOutput5 = generateHCL(hcl5);
assert(hclOutput5.includes('default_model = "openai/kimi-k2.5"'), 'default_model should be bare attribute');
assert(hclOutput5.includes('providers {'), 'providers should be HCL block');
assert(hclOutput5.includes('name = "openai"'), 'provider label as name attr');
assert(hclOutput5.includes('models {'), 'models should be HCL block');
assert(hclOutput5.includes('id = "kimi-k2.5"'), 'model label as id attr');
console.log('Generated HCL:\n' + hclOutput5);

console.log('\n=== Test generateHCL: api_key with special chars ===');
const hcl6 = new DocumentBuilder()
    .block(
        new BlockBuilder('models')
            .label('kimi-k2.5')
            .attr('api_key', string('sk-ZaH1YnkiGmcBt8qxKWfsBV5w9aInp4QuDUeq1HEIOAzEg5cT'))
            .build()
    )
    .build();
const hclOutput6 = generateHCL(hcl6);
assert(hclOutput6.includes('api_key = "sk-ZaH1YnkiGmcBt8qxKWfsBV5w9aInp4QuDUeq1HEIOAzEg5cT"'), 'api_key with - should be quoted');
console.log('Generated HCL:\n' + hclOutput6);

console.log('\n=== Test generateHCL: empty attributes ===');
const hcl7 = new DocumentBuilder()
    .block(new BlockBuilder('test_block').build())
    .build();
const hclOutput7 = generateHCL(hcl7);
assert(hclOutput7.includes('test_block { }'), 'empty block should be { }');
console.log('Generated HCL:\n' + hclOutput7);

console.log('\n=== Test generate vs generateHCL difference ===');
const mixedDoc = new DocumentBuilder()
    .block(
        new BlockBuilder('providers')
            .label('openai')
            .attr('api_key', string('sk-xxx'))
            .build()
    )
    .build();
const genOutput = generate(mixedDoc);
const hclGenOutput = generateHCL(mixedDoc);
assert(genOutput.includes('providers "openai"'), 'generate should use ACL-style labeled block');
assert(hclGenOutput.includes('providers {'), 'generateHCL should use HCL-style anonymous block');
assert(hclGenOutput.includes('name = "openai"'), 'generateHCL should output label as name attr');
console.log('generate():\n' + genOutput);
console.log('generateHCL():\n' + hclGenOutput);

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
