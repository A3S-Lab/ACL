/**
 * ACL Generator - Generates ACL text from AST
 */

/**
 * @typedef {Object} GeneratorConfig
 * @property {boolean} [labelsAsAttrs=false] - Output labels as attributes (HCL format) instead of block labels (ACL format)
 */

function needsQuotes(s) {
  if (s === '') return true;
  if (s.includes(' ') || s.includes('#') || s.includes('"') || s.includes("'") ||
      s.includes('\\') || s.includes('\n') || s.includes('\r') || s.includes('\t') ||
      s.includes(':') || s.includes('=') || s.includes('{') || s.includes('}') ||
      s.includes('[') || s.includes(']') || s.startsWith('-') || s.startsWith('.') ||
      s === 'true' || s === 'false' || s === 'null') {
    return true;
  }
  if (s.length > 0 && s[0] >= '0' && s[0] <= '9') return true;
  // For HCL attribute values, always quote to avoid interpretation as identifier
  // Model IDs with / or - should be quoted
  if (s.includes('/') || s.includes('-')) return true;
  return false;
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
      const pairStrs = value.pairs.map(([k, v]) => `${k} = ${writeValue(v, indent)}`);
      return `{${pairStrs.join(', ')}}`;
    case 'Call':
      const args = value.args.map(a => writeValue(a, indent)).join(', ');
      return `${value.name}(${args})`;
  }
}

function generateBlock(block, indent = 0, config = {}) {
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

  // In labelsAsAttrs mode, don't output labels in block header
  // Instead, we'll output them as attributes inside the block
  if (!config.labelsAsAttrs && block.labels.length > 0) {
    result += `${block.name} ${block.labels.map(l => `"${escapeString(l)}"`).join(' ')}`;
  } else {
    result += block.name;
  }

  const attrKeys = Array.from(block.attributes.keys()).sort();

  // In labelsAsAttrs mode, output the first label as a "name" attribute
  const hasLabelsAsAttrs = config.labelsAsAttrs && block.labels.length > 0;

  if (attrKeys.length === 0 && block.blocks.length === 0 && !hasLabelsAsAttrs) {
    result += ' { }';
    return result;
  }

  result += ' {\n';

  // In labelsAsAttrs mode, output the first label as an attribute
  // For "models" blocks, use "id" as the attribute name (HCL format)
  // For other blocks (like "providers"), use "name"
  // Always quote the value in HCL format to avoid interpretation as identifier
  if (hasLabelsAsAttrs) {
    const labelValue = block.labels[0];
    const attrName = block.name === 'models' ? 'id' : 'name';
    result += `${spaces}  ${attrName} = "${escapeString(labelValue)}"\n`;
  }

  for (const key of attrKeys) {
    const value = block.attributes.get(key);
    result += `${spaces}  ${key} = ${writeValue(value, indent + 1)}\n`;
  }

  for (const nested of block.blocks) {
    result += generateBlock(nested, indent + 1, config) + '\n';
  }

  result += `${spaces}}`;
  return result;
}

/**
 * Generate ACL text from a Document
 * @param {Object} doc - The document to generate
 * @param {GeneratorConfig} [config] - Optional generator configuration
 * @returns {string} The generated text
 */
function generate(doc, config = {}) {
  return doc.blocks.map(b => generateBlock(b, 0, config)).join('\n');
}

/**
 * Generate HCL-formatted text from a Document (labels as attributes)
 * @param {Object} doc - The document to generate
 * @returns {string} The generated HCL-formatted text
 */
function generateHCL(doc) {
  return generate(doc, { labelsAsAttrs: true });
}

module.exports = { generate, generateHCL, generateBlock, writeValue, needsQuotes, escapeString };
