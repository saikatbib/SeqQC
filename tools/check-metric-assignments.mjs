import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

function parseCsv(text) {
  const rows = [];
  let row = [];
  let field = '';
  let quoted = false;
  for (let index = 0; index < text.length; index += 1) {
    const character = text[index];
    if (quoted) {
      if (character === '"' && text[index + 1] === '"') {
        field += '"';
        index += 1;
      } else if (character === '"') quoted = false;
      else field += character;
    } else if (character === '"') quoted = true;
    else if (character === ',') {
      row.push(field);
      field = '';
    } else if (character === '\n') {
      row.push(field.replace(/\r$/, ''));
      rows.push(row);
      row = [];
      field = '';
    } else field += character;
  }
  if (field.length || row.length) {
    row.push(field);
    rows.push(row);
  }
  const [headers, ...values] = rows.filter((item) => item.some((fieldValue) => fieldValue.length));
  return values.map((columns) => Object.fromEntries(headers.map((header, index) => [header, columns[index] ?? ''])));
}

function matches(metricId, matcher) {
  if (matcher.exact) return metricId === matcher.exact;
  if (matcher.prefix) return metricId.startsWith(matcher.prefix);
  throw new Error(`Unsupported metric match rule: ${JSON.stringify(matcher)}`);
}

export async function checkMetricAssignments() {
  const registryPath = path.join(repositoryRoot, 'docs', 'planning', '02a-metric-registry.csv');
  const rulesPath = path.join(repositoryRoot, 'config', 'metric-implementation-rules.json');
  const metrics = parseCsv(await readFile(registryPath, 'utf8'));
  const configuration = JSON.parse(await readFile(rulesPath, 'utf8'));
  const ids = new Set();
  const counts = new Map();
  const assignments = [];

  for (const metric of metrics) {
    if (!metric.metric_id) throw new Error('Metric row is missing metric_id');
    if (ids.has(metric.metric_id)) throw new Error(`Duplicate metric ID: ${metric.metric_id}`);
    ids.add(metric.metric_id);
    const matched = configuration.rules.filter((rule) => matches(metric.metric_id, rule.match));
    if (!matched.length) throw new Error(`No implementation assignment for ${metric.metric_id}`);
    const rule = matched[0];
    if (!rule.owner || !rule.primary_method || !rule.validation_reference || !rule.release_stage || !rule.platforms?.length) {
      throw new Error(`Incomplete implementation rule: ${rule.id}`);
    }
    assignments.push({ metric_id: metric.metric_id, rule_id: rule.id, owner: rule.owner });
    counts.set(rule.id, (counts.get(rule.id) ?? 0) + 1);
  }

  for (const rule of configuration.rules) {
    if (!counts.has(rule.id)) throw new Error(`Implementation rule matches no metrics: ${rule.id}`);
  }

  return { metrics: metrics.length, rules: configuration.rules.length, counts: Object.fromEntries(counts), assignments };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const result = await checkMetricAssignments();
  process.stdout.write(`Metric assignment check passed: ${result.metrics} metrics across ${result.rules} rules.\n`);
  for (const [rule, count] of Object.entries(result.counts)) process.stdout.write(`  ${rule}: ${count}\n`);
}

