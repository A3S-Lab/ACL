function normalizeSchema(schema, path = '$schema', ancestors = new Set()) {
  assertRecord(schema, path);
  rejectCycle(schema, path, ancestors);
  assertKeys(
    schema,
    ['attributes', 'blocks', 'allowUnknownAttributes', 'allowUnknownBlocks'],
    path
  );
  const normalized = {
    attributes: normalizeNamedRules(
      schema.attributes ?? {},
      `${path}.attributes`,
      normalizeAttribute,
      ancestors
    ),
    blocks: normalizeNamedRules(
      schema.blocks ?? {},
      `${path}.blocks`,
      normalizeBlock,
      ancestors
    ),
    allowUnknownAttributes: optionalBoolean(
      schema.allowUnknownAttributes,
      false,
      `${path}.allowUnknownAttributes`
    ),
    allowUnknownBlocks: optionalBoolean(
      schema.allowUnknownBlocks,
      false,
      `${path}.allowUnknownBlocks`
    ),
  };
  ancestors.delete(schema);
  return normalized;
}

function normalizeNamedRules(value, path, normalize, ancestors) {
  assertRecord(value, path);
  const rules = new Map();
  const entries = Object.entries(value)
    .sort(([left], [right]) => compareStrings(left, right));
  for (const [name, rule] of entries) {
    rules.set(name, normalize(rule, `${path}.${name}`, ancestors));
  }
  return rules;
}

function normalizeAttribute(attribute, path, ancestors) {
  assertRecord(attribute, path);
  rejectCycle(attribute, path, ancestors);
  assertKeys(attribute, ['required', 'value'], path);
  if (attribute.value === undefined) {
    throw new TypeError(`${path}.value is required`);
  }
  const normalized = {
    required: optionalBoolean(attribute.required, false, `${path}.required`),
    value: normalizeValue(attribute.value, `${path}.value`, ancestors),
  };
  ancestors.delete(attribute);
  return normalized;
}

function normalizeBlock(block, path, ancestors) {
  assertRecord(block, path);
  rejectCycle(block, path, ancestors);
  assertKeys(block, ['occurrences', 'labels', 'body', 'unordered'], path);
  const normalized = {
    occurrences: normalizeCardinality(
      block.occurrences ?? { min: 0, max: null },
      `${path}.occurrences`
    ),
    labels: normalizeCardinality(
      block.labels ?? { min: 0, max: 0 },
      `${path}.labels`
    ),
    unordered: optionalBoolean(block.unordered, false, `${path}.unordered`),
    body: normalizeSchema(block.body ?? {}, `${path}.body`, ancestors),
  };
  ancestors.delete(block);
  return normalized;
}

function normalizeValue(value, path, ancestors) {
  assertRecord(value, path);
  rejectCycle(value, path, ancestors);
  if (typeof value.kind !== 'string') {
    throw new TypeError(`${path}.kind must be a string`);
  }

  let normalized;
  switch (value.kind) {
    case 'Any':
    case 'String':
    case 'Number':
    case 'Bool':
    case 'Null':
      assertKeys(value, ['kind'], path);
      normalized = { kind: value.kind };
      break;
    case 'List':
      assertKeys(value, ['kind', 'item'], path);
      if (value.item === undefined) throw new TypeError(`${path}.item is required`);
      normalized = {
        kind: 'List',
        item: normalizeValue(value.item, `${path}.item`, ancestors),
      };
      break;
    case 'Object':
      assertKeys(value, ['kind', 'fields', 'allowUnknownFields'], path);
      normalized = {
        kind: 'Object',
        fields: normalizeNamedRules(
          value.fields ?? {},
          `${path}.fields`,
          normalizeAttribute,
          ancestors
        ),
        allowUnknownFields: optionalBoolean(
          value.allowUnknownFields,
          false,
          `${path}.allowUnknownFields`
        ),
      };
      break;
    case 'Call': {
      assertKeys(value, ['kind', 'names', 'arguments', 'argument'], path);
      const names = value.names ?? [];
      if (!Array.isArray(names) || names.some((name) => typeof name !== 'string')) {
        throw new TypeError(`${path}.names must be an array of strings`);
      }
      normalized = {
        kind: 'Call',
        names: new Set(names),
        arguments: normalizeCardinality(
          value.arguments ?? { min: 0, max: null },
          `${path}.arguments`
        ),
        argument: normalizeValue(
          value.argument ?? { kind: 'Any' },
          `${path}.argument`,
          ancestors
        ),
      };
      break;
    }
    case 'OneOf':
      assertKeys(value, ['kind', 'variants'], path);
      if (!Array.isArray(value.variants) || value.variants.length === 0) {
        throw new TypeError(`${path}.variants must contain at least one schema`);
      }
      normalized = {
        kind: 'OneOf',
        variants: value.variants.map((variant, index) =>
          normalizeValue(variant, `${path}.variants[${index}]`, ancestors)
        ),
      };
      break;
    default:
      throw new TypeError(`${path}.kind is not a supported ACL value schema`);
  }
  ancestors.delete(value);
  return normalized;
}

function normalizeCardinality(cardinality, path) {
  assertRecord(cardinality, path);
  assertKeys(cardinality, ['min', 'max'], path);
  if (!Number.isSafeInteger(cardinality.min) || cardinality.min < 0) {
    throw new TypeError(`${path}.min must be a non-negative safe integer`);
  }
  const max = cardinality.max ?? null;
  if (max !== null && (!Number.isSafeInteger(max) || max < cardinality.min)) {
    throw new TypeError(
      `${path}.max must be null or a safe integer greater than or equal to min`
    );
  }
  return { min: cardinality.min, max };
}

function compareStrings(left, right) {
  const leftScalars = Array.from(left);
  const rightScalars = Array.from(right);
  const length = Math.min(leftScalars.length, rightScalars.length);
  for (let index = 0; index < length; index++) {
    const difference = leftScalars[index].codePointAt(0) - rightScalars[index].codePointAt(0);
    if (difference !== 0) return difference;
  }
  return leftScalars.length - rightScalars.length;
}

function assertRecord(value, path) {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) {
    throw new TypeError(`${path} must be an object`);
  }
}

function assertKeys(value, allowed, path) {
  const allowedKeys = new Set(allowed);
  const unknown = Object.keys(value).find((key) => !allowedKeys.has(key));
  if (unknown !== undefined) {
    throw new TypeError(`${path}.${unknown} is not a supported schema field`);
  }
}

function rejectCycle(value, path, ancestors) {
  if (ancestors.has(value)) throw new TypeError(`${path} contains a schema cycle`);
  ancestors.add(value);
}

function optionalBoolean(value, fallback, path) {
  if (value === undefined) return fallback;
  if (typeof value !== 'boolean') throw new TypeError(`${path} must be a boolean`);
  return value;
}

module.exports = {
  compareStrings,
  normalizeSchema,
};
