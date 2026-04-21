/**
 * ACL SDK Tests
 */

const { parse, generate, generateHCL, string, number, boolean, call, BlockBuilder, DocumentBuilder } = require('./index.js');

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

console.log('\n=== All tests passed! ===');
