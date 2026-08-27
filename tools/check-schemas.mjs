import { readFile, readdir } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const schemaRoot = path.join(repositoryRoot, 'schemas', 'v1');
const exampleRoot = path.join(repositoryRoot, 'schemas', 'examples');
const invalidExampleRoot = path.join(repositoryRoot, 'schemas', 'examples-invalid');

async function jsonFiles(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const target = path.join(directory, entry.name);
    if (entry.isDirectory()) files.push(...await jsonFiles(target));
    if (entry.isFile() && entry.name.endsWith('.json')) files.push(target);
  }
  return files.sort();
}

function walk(value, visit) {
  if (Array.isArray(value)) {
    value.forEach((item) => walk(item, visit));
    return;
  }
  if (value && typeof value === 'object') {
    visit(value);
    Object.values(value).forEach((item) => walk(item, visit));
  }
}

function decodePointerPart(part) {
  return part.replaceAll('~1', '/').replaceAll('~0', '~');
}

function pointerValue(document, fragment) {
  if (!fragment || fragment === '#') return document;
  if (!fragment.startsWith('#/')) throw new Error(`Unsupported JSON pointer: ${fragment}`);
  let current = document;
  for (const rawPart of fragment.slice(2).split('/')) {
    const part = decodePointerPart(rawPart);
    if (!current || typeof current !== 'object' || !(part in current)) {
      throw new Error(`JSON pointer does not resolve: ${fragment}`);
    }
    current = current[part];
  }
  return current;
}

function actualType(value) {
  if (value === null) return 'null';
  if (Array.isArray(value)) return 'array';
  if (Number.isInteger(value)) return 'integer';
  if (typeof value === 'number') return 'number';
  return typeof value;
}

function deepEqual(left, right) {
  return JSON.stringify(left) === JSON.stringify(right);
}

function resolveReference(reference, currentFile, documents) {
  if (/^[a-z][a-z0-9+.-]*:/i.test(reference)) {
    throw new Error(`Runtime-external schema reference is forbidden: ${reference}`);
  }
  const [relativeFile, rawFragment] = reference.split('#', 2);
  const targetFile = relativeFile ? path.resolve(path.dirname(currentFile), relativeFile) : currentFile;
  const targetDocument = documents.get(targetFile);
  if (!targetDocument) throw new Error(`Missing local schema reference: ${reference} from ${currentFile}`);
  const fragment = rawFragment === undefined ? '' : `#${rawFragment}`;
  return { schema: pointerValue(targetDocument, fragment), file: targetFile };
}

function validate(value, schema, currentFile, documents, location = '$') {
  const errors = [];
  const add = (message) => errors.push(`${location}: ${message}`);

  if (schema.$ref) {
    const resolved = resolveReference(schema.$ref, currentFile, documents);
    return validate(value, resolved.schema, resolved.file, documents, location);
  }

  if (schema.allOf) {
    for (const child of schema.allOf) errors.push(...validate(value, child, currentFile, documents, location));
  }
  if (schema.oneOf) {
    const results = schema.oneOf.map((child) => validate(value, child, currentFile, documents, location));
    if (results.filter((result) => result.length === 0).length !== 1) add('must match exactly one oneOf branch');
  }
  if (schema.if) {
    const conditionErrors = validate(value, schema.if, currentFile, documents, location);
    if (conditionErrors.length === 0 && schema.then) {
      errors.push(...validate(value, schema.then, currentFile, documents, location));
    }
  }

  if ('const' in schema && !deepEqual(value, schema.const)) add(`must equal ${JSON.stringify(schema.const)}`);
  if (schema.enum && !schema.enum.some((item) => deepEqual(item, value))) add('must be an allowed enum value');

  if (schema.type) {
    const allowed = Array.isArray(schema.type) ? schema.type : [schema.type];
    const observed = actualType(value);
    const numberAcceptsInteger = allowed.includes('number') && observed === 'integer';
    if (!allowed.includes(observed) && !numberAcceptsInteger) add(`expected ${allowed.join('|')} but found ${observed}`);
  }

  if (typeof value === 'string') {
    if (schema.minLength !== undefined && value.length < schema.minLength) add(`minimum length is ${schema.minLength}`);
    if (schema.maxLength !== undefined && value.length > schema.maxLength) add(`maximum length is ${schema.maxLength}`);
    if (schema.pattern && !(new RegExp(schema.pattern).test(value))) add(`does not match ${schema.pattern}`);
    if (schema.format === 'uuid' && !/^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i.test(value)) add('invalid UUID');
    if (schema.format === 'date-time' && Number.isNaN(Date.parse(value))) add('invalid date-time');
  }

  if (typeof value === 'number') {
    if (schema.minimum !== undefined && value < schema.minimum) add(`minimum is ${schema.minimum}`);
    if (schema.maximum !== undefined && value > schema.maximum) add(`maximum is ${schema.maximum}`);
  }

  if (Array.isArray(value)) {
    if (schema.minItems !== undefined && value.length < schema.minItems) add(`minimum items is ${schema.minItems}`);
    if (schema.uniqueItems && new Set(value.map((item) => JSON.stringify(item))).size !== value.length) add('items must be unique');
    if (schema.items) value.forEach((item, index) => errors.push(...validate(item, schema.items, currentFile, documents, `${location}[${index}]`)));
  }

  if (value && typeof value === 'object' && !Array.isArray(value)) {
    for (const required of schema.required ?? []) {
      if (!(required in value)) add(`missing required property ${required}`);
    }
    const known = new Set(Object.keys(schema.properties ?? {}));
    if (schema.additionalProperties === false) {
      for (const key of Object.keys(value)) if (!known.has(key)) add(`unknown property ${key}`);
    }
    for (const [key, child] of Object.entries(schema.properties ?? {})) {
      if (key in value) errors.push(...validate(value[key], child, currentFile, documents, `${location}.${key}`));
    }
  }

  return errors;
}

export async function checkSchemas() {
  const schemaFiles = await jsonFiles(schemaRoot);
  const documents = new Map();
  for (const file of schemaFiles) {
    const document = JSON.parse(await readFile(file, 'utf8'));
    documents.set(path.resolve(file), document);
    if (document.$schema !== 'https://json-schema.org/draft/2020-12/schema') throw new Error(`${file} does not declare Draft 2020-12`);
    if (!document.$id?.startsWith('https://schemas.seqqc.org/v1/')) throw new Error(`${file} has an invalid SeqQC schema ID`);
    if (!document.title) throw new Error(`${file} has no title`);
  }

  for (const [file, document] of documents) {
    walk(document, (node) => {
      if (node.$ref) resolveReference(node.$ref, file, documents);
    });
  }

  const examples = await jsonFiles(exampleRoot);
  for (const exampleFile of examples) {
    const match = path.basename(exampleFile).match(/^([a-z-]+)\.valid\.json$/);
    if (!match) throw new Error(`Example name must be <schema>.valid.json: ${exampleFile}`);
    const schemaFile = path.resolve(schemaRoot, `${match[1]}.schema.json`);
    const schema = documents.get(schemaFile);
    if (!schema) throw new Error(`No schema for example ${exampleFile}`);
    const value = JSON.parse(await readFile(exampleFile, 'utf8'));
    const errors = validate(value, schema, schemaFile, documents);
    if (errors.length) throw new Error(`${exampleFile} failed:\n${errors.join('\n')}`);
  }

  const invalidExamples = await jsonFiles(invalidExampleRoot);
  for (const exampleFile of invalidExamples) {
    const match = path.basename(exampleFile).match(/^([a-z-]+)\.invalid\.[a-z0-9-]+\.json$/);
    if (!match) throw new Error(`Invalid example name must be <schema>.invalid.<case>.json: ${exampleFile}`);
    const schemaFile = path.resolve(schemaRoot, `${match[1]}.schema.json`);
    const schema = documents.get(schemaFile);
    if (!schema) throw new Error(`No schema for invalid example ${exampleFile}`);
    const value = JSON.parse(await readFile(exampleFile, 'utf8'));
    const errors = validate(value, schema, schemaFile, documents);
    if (!errors.length) throw new Error(`${exampleFile} was expected to fail but passed`);
  }

  return { schemas: schemaFiles.length, examples: examples.length, invalidExamples: invalidExamples.length };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const result = await checkSchemas();
  process.stdout.write(`Schema check passed: ${result.schemas} schemas, ${result.examples} valid examples, and ${result.invalidExamples} invalid examples.\n`);
}
