/**
 * ACL Generator - Generates ACL text from AST
 */

function needsQuotes() {
  // Every String AST value must stay quoted. Numeric-looking strings would
  // otherwise be reparsed as Number values.
  return true;
}

function escapeString(s) {
  let result = '';
  for (const c of s) {
    switch (c) {
      case '"': result += '\\"'; break;
      case '\\': result += '\\\\'; break;
      case '\n': result += '\\n'; break;
      case '\r': result += '\\r'; break;
      case '\t': result += '\\t'; break;
      default: result += c;
    }
  }
  return result;
}

function writeValue(value, indent = 0) {
  switch (value.kind) {
    case 'String':
      return needsQuotes(value.value) ? `"${escapeString(value.value)}"` : value.value;
    case 'Number':
      return String(value.value);
    case 'Bool':
      return String(value.value);
    case 'Null':
      return 'null';
    case 'List':
      if (value.items.length === 0) return '[]';
      return `[${value.items.map(v => writeValue(v, indent)).join(', ')}]`;
    case 'Object':
      if (value.pairs.length === 0) return '{}';
      const canonicalPairs = Array.from(new Map(value.pairs).entries())
        .sort(([left], [right]) => left < right ? -1 : left > right ? 1 : 0);
      const pairStrs = canonicalPairs.map(([k, v]) => `${k} = ${writeValue(v, indent)}`);
      return `{${pairStrs.join(', ')}}`;
    case 'Call':
      const args = value.args.map(a => writeValue(a, indent)).join(', ');
      return `${value.name}(${args})`;
  }
}

function generateBlock(block, indent = 0) {
  const spaces = '  '.repeat(indent);
  let result = spaces;

  // Special case: single-value block with no labels and no nested blocks
  // Output as bare attribute: name = value
  if (block.labels.length === 0 && block.blocks.length === 0 && block.attributes.size === 1) {
    const [key, value] = Array.from(block.attributes.entries())[0];
    if (value.kind === 'String' && !value.value.includes(' ')) {
      result += block.name + ' = ' + writeValue(value, indent) + '\n';
      return result;
    }
  }

  if (block.labels.length > 0) {
    result += `${block.name} ${block.labels.map(l => `"${escapeString(l)}"`).join(' ')}`;
  } else {
    result += block.name;
  }

  const attrKeys = Array.from(block.attributes.keys()).sort();

  if (attrKeys.length === 0 && block.blocks.length === 0) {
    result += ' { }';
    return result;
  }

  result += ' {\n';

  for (const key of attrKeys) {
    const value = block.attributes.get(key);
    result += `${spaces}  ${key} = ${writeValue(value, indent + 1)}\n`;
  }

  for (const nested of block.blocks) {
    result += generateBlock(nested, indent + 1) + '\n';
  }

  result += `${spaces}}`;
  return result;
}

/**
 * Generate ACL text from a Document
 * @param {Object} doc - The document to generate
 * @returns {string} The generated text
 */
function generate(doc, options) {
  if (options !== undefined) {
    throw new TypeError('ACL generator options are not supported');
  }
  return doc.blocks.map(b => generateBlock(b)).join('\n');
}

module.exports = { generate, generateBlock, writeValue, needsQuotes, escapeString };
