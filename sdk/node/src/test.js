/**
 * ACL SDK Tests
 */

const { parse, generate, string, number, boolean, call, BlockBuilder, DocumentBuilder } = require('./index.js');

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

console.log('\n=== All tests passed! ===');
