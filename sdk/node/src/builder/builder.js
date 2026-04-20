/**
 * ACL Builder API - Convenient way to construct ACL documents
 */

// Value constructors
function string(s) {
  return { kind: 'String', value: s };
}

function number(n) {
  return { kind: 'Number', value: n };
}

function boolean(b) {
  return { kind: 'Bool', value: b };
}

function bool(b) {
  return { kind: 'Bool', value: b };
}

function nullValue() {
  return { kind: 'Null' };
}

function list(items) {
  return { kind: 'List', items };
}

function call(name, args) {
  return { kind: 'Call', name, args };
}

// BlockBuilder
class BlockBuilder {
  constructor(name) {
    this.name = name;
    this.labels = [];
    this.blocks = [];
    this.attributes = new Map();
  }

  label(label) {
    this.labels.push(label);
    return this;
  }

  attr(key, value) {
    this.attributes.set(key, value);
    return this;
  }

  nestedBlock(block) {
    this.blocks.push(block);
    return this;
  }

  build() {
    return {
      name: this.name,
      labels: this.labels,
      blocks: this.blocks,
      attributes: this.attributes,
    };
  }
}

// DocumentBuilder
class DocumentBuilder {
  constructor() {
    this.blocks = [];
  }

  block(block) {
    this.blocks.push(block);
    return this;
  }

  kvBlock(name, key, value) {
    const block = {
      name,
      labels: [],
      blocks: [],
      attributes: new Map([[key, value]]),
    };
    this.blocks.push(block);
    return this;
  }

  build() {
    return { blocks: this.blocks };
  }
}

module.exports = {
  string,
  number,
  boolean,
  bool,
  nullValue,
  list,
  call,
  BlockBuilder,
  DocumentBuilder,
};
