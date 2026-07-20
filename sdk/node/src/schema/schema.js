const { normalizeParseLimits } = require('../parser/Parser.js');
const {
  compareStrings,
  normalizeSchema,
} = require('./definition.js');

const SCHEMA_DIAGNOSTIC_CODES = Object.freeze({
  UNKNOWN_ATTRIBUTE: 'acl.schema.unknown_attribute',
  DUPLICATE_ATTRIBUTE: 'acl.schema.duplicate_attribute',
  MISSING_ATTRIBUTE: 'acl.schema.missing_attribute',
  UNKNOWN_BLOCK: 'acl.schema.unknown_block',
  BLOCK_COUNT: 'acl.schema.block_count',
  LABEL_COUNT: 'acl.schema.label_count',
  VALUE_TYPE: 'acl.schema.value_type',
  UNKNOWN_OBJECT_FIELD: 'acl.schema.unknown_object_field',
  DUPLICATE_OBJECT_FIELD: 'acl.schema.duplicate_object_field',
  MISSING_OBJECT_FIELD: 'acl.schema.missing_object_field',
  CALL_NAME: 'acl.schema.call_name',
  CALL_ARGUMENT_COUNT: 'acl.schema.call_argument_count',
});

function validateDocument(document, schema, limits) {
  const normalizedLimits = normalizeParseLimits(limits);
  const normalizedSchema = normalizeSchema(schema, '$schema', new Set());
  assertDocument(document);
  const validator = new Validator(normalizedLimits.maxDiagnostics);
  validator.validateDocument(document, normalizedSchema);
  return validator.report;
}

class Validator {
  constructor(maxDiagnostics) {
    this.maxDiagnostics = maxDiagnostics;
    this.report = { diagnostics: [], truncated: false };
  }

  validateDocument(document, schema) {
    const attributes = new Map();
    const blocks = [];
    for (const block of document.blocks) {
      const attribute = bareAttribute(block);
      if (attribute) {
        append(attributes, attribute.name, attribute.value);
      } else {
        blocks.push(block);
      }
    }
    return this.validateBody(attributes, blocks, schema, '$');
  }

  validateBody(attributes, blocks, schema, path) {
    for (const [name, attributeSchema] of schema.attributes) {
      if (attributeSchema.required && !attributes.has(name)) {
        if (!this.record(
          SCHEMA_DIAGNOSTIC_CODES.MISSING_ATTRIBUTE,
          'Required attribute is missing',
          `${path}.attributes.${name}`
        )) return false;
      }
    }

    for (const name of Array.from(attributes.keys()).sort(compareStrings)) {
      const values = attributes.get(name);
      const attributePath = `${path}.attributes.${name}`;
      if (values.length > 1 && !this.record(
        SCHEMA_DIAGNOSTIC_CODES.DUPLICATE_ATTRIBUTE,
        'Attribute appears more than once',
        attributePath
      )) return false;

      const attributeSchema = schema.attributes.get(name);
      if (attributeSchema) {
        if (!this.validateValue(values[values.length - 1], attributeSchema.value, attributePath)) {
          return false;
        }
      } else if (!schema.allowUnknownAttributes && !this.record(
        SCHEMA_DIAGNOSTIC_CODES.UNKNOWN_ATTRIBUTE,
        'Attribute is not allowed by the schema',
        attributePath
      )) {
        return false;
      }
    }

    const counts = new Map();
    for (const block of blocks) {
      counts.set(block.name, (counts.get(block.name) ?? 0) + 1);
    }
    for (const [name, blockSchema] of schema.blocks) {
      const count = counts.get(name) ?? 0;
      if (!contains(blockSchema.occurrences, count) && !this.record(
        SCHEMA_DIAGNOSTIC_CODES.BLOCK_COUNT,
        `Expected ${describe(blockSchema.occurrences, 'block', 'blocks')}, found ${count}`,
        `${path}.blocks.${name}`
      )) {
        return false;
      }
    }

    const indexes = new Map();
    for (const block of blocks) {
      const index = indexes.get(block.name) ?? 0;
      indexes.set(block.name, index + 1);
      const blockPath = `${path}.blocks.${block.name}[${index}]`;
      const blockSchema = schema.blocks.get(block.name);
      if (blockSchema) {
        if (!this.validateBlock(block, blockSchema, blockPath)) return false;
      } else if (!schema.allowUnknownBlocks && !this.record(
        SCHEMA_DIAGNOSTIC_CODES.UNKNOWN_BLOCK,
        'Block is not allowed by the schema',
        blockPath
      )) {
        return false;
      }
    }
    return true;
  }

  validateBlock(block, schema, path) {
    if (!contains(schema.labels, block.labels.length) && !this.record(
      SCHEMA_DIAGNOSTIC_CODES.LABEL_COUNT,
      `Expected ${describe(schema.labels, 'label', 'labels')}, found ${block.labels.length}`,
      `${path}.labels`
    )) {
      return false;
    }

    const attributes = new Map(
      Array.from(block.attributes.entries()).map(([name, value]) => [name, [value]])
    );
    return this.validateBody(attributes, block.blocks, schema.body, path);
  }

  validateValue(value, schema, path) {
    switch (schema.kind) {
      case 'Any':
        return true;
      case 'String':
      case 'Number':
      case 'Bool':
      case 'Null':
        return value.kind === schema.kind || this.typeError(value, schema.kind, path);
      case 'List':
        if (value.kind !== 'List') return this.typeError(value, 'List', path);
        for (const [index, item] of value.items.entries()) {
          if (!this.validateValue(item, schema.item, `${path}.items[${index}]`)) return false;
        }
        return true;
      case 'Object':
        if (value.kind !== 'Object') return this.typeError(value, 'Object', path);
        return this.validateObject(value.pairs, schema, path);
      case 'Call':
        if (value.kind !== 'Call') return this.typeError(value, 'Call', path);
        return this.validateCall(value.name, value.args, schema, path);
      case 'OneOf':
        if (schema.variants.some((variant) => valueMatches(value, variant))) return true;
        return this.record(
          SCHEMA_DIAGNOSTIC_CODES.VALUE_TYPE,
          'Value does not match any allowed schema variant',
          path
        );
      default:
        throw new TypeError('normalized ACL value schema has an unsupported kind');
    }
  }

  typeError(value, expected, path) {
    return this.record(
      SCHEMA_DIAGNOSTIC_CODES.VALUE_TYPE,
      `Expected ${expected}, found ${value.kind}`,
      path
    );
  }

  validateObject(pairs, schema, path) {
    const fields = new Map();
    for (const [name, value] of pairs) append(fields, name, value);

    for (const [name, fieldSchema] of schema.fields) {
      if (fieldSchema.required && !fields.has(name)) {
        if (!this.record(
          SCHEMA_DIAGNOSTIC_CODES.MISSING_OBJECT_FIELD,
          'Required object field is missing',
          `${path}.fields.${name}`
        )) return false;
      }
    }

    for (const name of Array.from(fields.keys()).sort(compareStrings)) {
      const values = fields.get(name);
      const fieldPath = `${path}.fields.${name}`;
      if (values.length > 1 && !this.record(
        SCHEMA_DIAGNOSTIC_CODES.DUPLICATE_OBJECT_FIELD,
        'Object field appears more than once',
        fieldPath
      )) return false;

      const fieldSchema = schema.fields.get(name);
      if (fieldSchema) {
        if (!this.validateValue(values[values.length - 1], fieldSchema.value, fieldPath)) {
          return false;
        }
      } else if (!schema.allowUnknownFields && !this.record(
        SCHEMA_DIAGNOSTIC_CODES.UNKNOWN_OBJECT_FIELD,
        'Object field is not allowed by the schema',
        fieldPath
      )) {
        return false;
      }
    }
    return true;
  }

  validateCall(name, args, schema, path) {
    if (schema.names.size > 0 && !schema.names.has(name) && !this.record(
      SCHEMA_DIAGNOSTIC_CODES.CALL_NAME,
      'Call function is not allowed by the schema',
      `${path}.function`
    )) {
      return false;
    }
    if (!contains(schema.arguments, args.length) && !this.record(
      SCHEMA_DIAGNOSTIC_CODES.CALL_ARGUMENT_COUNT,
      `Expected ${describe(schema.arguments, 'argument', 'arguments')}, found ${args.length}`,
      `${path}.arguments`
    )) {
      return false;
    }
    for (const [index, argument] of args.entries()) {
      if (!this.validateValue(argument, schema.argument, `${path}.arguments[${index}]`)) {
        return false;
      }
    }
    return true;
  }

  record(code, message, path) {
    if (this.report.diagnostics.length >= this.maxDiagnostics) {
      this.report.truncated = true;
      return false;
    }
    this.report.diagnostics.push({ code, message, path });
    return true;
  }
}

function valueMatches(value, schema) {
  switch (schema.kind) {
    case 'Any':
      return true;
    case 'String':
    case 'Number':
    case 'Bool':
    case 'Null':
      return value.kind === schema.kind;
    case 'List':
      return value.kind === 'List'
        && value.items.every((item) => valueMatches(item, schema.item));
    case 'Object':
      return value.kind === 'Object' && objectMatches(value.pairs, schema);
    case 'Call':
      return value.kind === 'Call' && callMatches(value.name, value.args, schema);
    case 'OneOf':
      return schema.variants.some((variant) => valueMatches(value, variant));
    default:
      return false;
  }
}

function objectMatches(pairs, schema) {
  const fields = new Map();
  for (const [name, value] of pairs) append(fields, name, value);
  for (const [name, fieldSchema] of schema.fields) {
    if (fieldSchema.required && !fields.has(name)) return false;
  }
  for (const [name, values] of fields) {
    if (values.length !== 1) return false;
    const fieldSchema = schema.fields.get(name);
    if (fieldSchema) {
      if (!valueMatches(values[0], fieldSchema.value)) return false;
    } else if (!schema.allowUnknownFields) {
      return false;
    }
  }
  return true;
}

function callMatches(name, args, schema) {
  return (schema.names.size === 0 || schema.names.has(name))
    && contains(schema.arguments, args.length)
    && args.every((argument) => valueMatches(argument, schema.argument));
}

function bareAttribute(block) {
  if (
    block.labels.length === 0
    && block.blocks.length === 0
    && block.attributes.size === 1
    && block.attributes.has(block.name)
  ) {
    return { name: block.name, value: block.attributes.get(block.name) };
  }
  return null;
}

function append(map, key, value) {
  const values = map.get(key);
  if (values) values.push(value);
  else map.set(key, [value]);
}

function contains(cardinality, count) {
  return count >= cardinality.min
    && (cardinality.max === null || count <= cardinality.max);
}

function describe(cardinality, singular, plural) {
  if (cardinality.max === cardinality.min) {
    return `exactly ${cardinality.min} ${cardinality.min === 1 ? singular : plural}`;
  }
  if (cardinality.max !== null) {
    return `between ${cardinality.min} and ${cardinality.max} ${plural}`;
  }
  return `at least ${cardinality.min} ${cardinality.min === 1 ? singular : plural}`;
}

function assertDocument(document) {
  if (!document || typeof document !== 'object' || !Array.isArray(document.blocks)) {
    throw new TypeError('ACL schema validation requires a Document');
  }
  const ancestors = new Set();
  for (const block of document.blocks) assertBlock(block, ancestors);
}

function assertBlock(block, ancestors) {
  if (
    !block
    || typeof block !== 'object'
    || typeof block.name !== 'string'
    || !Array.isArray(block.labels)
    || !Array.isArray(block.blocks)
    || !(block.attributes instanceof Map)
  ) {
    throw new TypeError('ACL schema validation received an invalid Block');
  }
  rejectDocumentCycle(block, ancestors);
  if (block.labels.some((label) => typeof label !== 'string')) {
    throw new TypeError('ACL schema validation received invalid Block labels');
  }
  for (const [name, value] of block.attributes) {
    if (typeof name !== 'string') {
      throw new TypeError('ACL schema validation received an invalid attribute name');
    }
    assertValue(value, ancestors);
  }
  for (const nested of block.blocks) assertBlock(nested, ancestors);
  ancestors.delete(block);
}

function assertValue(value, ancestors) {
  if (!value || typeof value !== 'object' || typeof value.kind !== 'string') {
    throw new TypeError('ACL schema validation received an invalid Value');
  }
  rejectDocumentCycle(value, ancestors);
  switch (value.kind) {
    case 'String':
      if (typeof value.value !== 'string') {
        throw new TypeError('ACL schema validation received an invalid String value');
      }
      break;
    case 'Number':
      if (typeof value.value !== 'number') {
        throw new TypeError('ACL schema validation received an invalid Number value');
      }
      break;
    case 'Bool':
      if (typeof value.value !== 'boolean') {
        throw new TypeError('ACL schema validation received an invalid Bool value');
      }
      break;
    case 'Null':
      break;
    case 'List':
      if (!Array.isArray(value.items)) {
        throw new TypeError('ACL schema validation received an invalid List value');
      }
      for (const item of value.items) assertValue(item, ancestors);
      break;
    case 'Object':
      if (
        !Array.isArray(value.pairs)
        || value.pairs.some((pair) =>
          !Array.isArray(pair) || pair.length !== 2 || typeof pair[0] !== 'string'
        )
      ) {
        throw new TypeError('ACL schema validation received an invalid Object value');
      }
      for (const [, item] of value.pairs) assertValue(item, ancestors);
      break;
    case 'Call':
      if (typeof value.name !== 'string' || !Array.isArray(value.args)) {
        throw new TypeError('ACL schema validation received an invalid Call value');
      }
      for (const argument of value.args) assertValue(argument, ancestors);
      break;
    default:
      throw new TypeError('ACL schema validation received an unsupported Value kind');
  }
  ancestors.delete(value);
}

function rejectDocumentCycle(value, ancestors) {
  if (ancestors.has(value)) {
    throw new TypeError('ACL schema validation received a cyclic Document');
  }
  ancestors.add(value);
}

module.exports = {
  SCHEMA_DIAGNOSTIC_CODES,
  validateDocument,
};
