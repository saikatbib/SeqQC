import { lstat, readdir } from 'node:fs/promises';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const supportedExtensions = [
  ['.fastq.gz', 'fastq'],
  ['.fq.gz', 'fastq'],
  ['.fastq', 'fastq'],
  ['.fq', 'fastq'],
  ['.bam', 'bam'],
  ['.cram', 'cram']
];

function classifyFile(name) {
  const lower = name.toLowerCase();
  for (const [extension, kind] of supportedExtensions) {
    if (lower.endsWith(extension)) return { kind, extension, stem: name.slice(0, -extension.length) };
  }
  return null;
}

function fastqIdentity(stem, relativeDirectory) {
  const patterns = [
    /^(.*?)[._-]R([12])(?:[._-]?(\d{3}))?$/i,
    /^(.*?)[._-]([12])$/i
  ];
  for (const pattern of patterns) {
    const match = stem.match(pattern);
    if (!match) continue;
    const pairStem = match[1];
    const laneMatch = pairStem.match(/(?:^|[_-])L(\d{3})(?:[_-]|$)/i);
    const sampleStem = pairStem.replace(/[_-]S\d+[_-]L\d{3}$/i, '').replace(/[_-]L\d{3}$/i, '');
    return {
      mate: Number(match[2]),
      pair_key: path.join(relativeDirectory, pairStem).toLowerCase(),
      proposed_sample: sampleStem,
      lane: laneMatch ? `L${laneMatch[1]}` : null,
      chunk: match[3] ?? null,
      confidence: pattern === patterns[0] ? 'high' : 'medium'
    };
  }
  return { mate: null, pair_key: null, proposed_sample: stem, lane: null, chunk: null, confidence: 'unresolved' };
}

async function walk(directory, root, result) {
  const entries = await readdir(directory, { withFileTypes: true });
  entries.sort((left, right) => left.name.localeCompare(right.name));
  for (const entry of entries) {
    const absolutePath = path.join(directory, entry.name);
    const relativePath = path.relative(root, absolutePath);
    if (entry.isSymbolicLink()) {
      result.ignored.push({ relative_path: relativePath, reason: 'symbolic_link' });
      continue;
    }
    if (entry.isDirectory()) {
      await walk(absolutePath, root, result);
      continue;
    }
    if (!entry.isFile()) continue;
    const classification = classifyFile(entry.name);
    if (!classification) {
      result.ignored.push({ relative_path: relativePath, reason: 'unsupported_extension' });
      continue;
    }
    const metadata = await lstat(absolutePath);
    const file = {
      relative_path: relativePath,
      kind: classification.kind,
      extension: classification.extension,
      size_bytes: metadata.size
    };
    if (classification.kind === 'fastq') {
      Object.assign(file, fastqIdentity(classification.stem, path.dirname(relativePath)));
    }
    result.files.push(file);
  }
}

function buildGroups(files) {
  const fastqGroups = new Map();
  const aligned = [];
  const unresolved = [];
  for (const file of files) {
    if (file.kind === 'bam' || file.kind === 'cram') {
      aligned.push({ kind: file.kind, files: [file.relative_path], status: 'aligned_input' });
      continue;
    }
    if (!file.pair_key || !file.mate) {
      unresolved.push({ kind: 'fastq', files: [file.relative_path], status: 'unresolved_layout' });
      continue;
    }
    const group = fastqGroups.get(file.pair_key) ?? {
      kind: 'fastq',
      pair_key: file.pair_key,
      proposed_sample: file.proposed_sample,
      lane: file.lane,
      r1: [],
      r2: [],
      confidence: file.confidence
    };
    group[`r${file.mate}`].push(file.relative_path);
    fastqGroups.set(file.pair_key, group);
  }

  const groups = [];
  for (const group of fastqGroups.values()) {
    const status = group.r1.length === 1 && group.r2.length === 1
      ? 'paired_candidate'
      : group.r1.length > 1 || group.r2.length > 1
        ? 'ambiguous_duplicate_mate'
        : 'missing_mate';
    groups.push({ ...group, status });
  }
  return [...groups, ...aligned, ...unresolved];
}

export async function discover(rootPath) {
  const root = path.resolve(rootPath);
  const metadata = await lstat(root);
  if (!metadata.isDirectory()) throw new Error('Discovery root must be a directory.');
  const result = { root, files: [], ignored: [] };
  await walk(root, root, result);
  return {
    ...result,
    groups: buildGroups(result.files),
    summary: {
      supported_files: result.files.length,
      ignored_files: result.ignored.length,
      fastq_files: result.files.filter((file) => file.kind === 'fastq').length,
      bam_files: result.files.filter((file) => file.kind === 'bam').length,
      cram_files: result.files.filter((file) => file.kind === 'cram').length
    }
  };
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  const root = process.argv[2];
  if (!root) {
    process.stderr.write('Usage: node prototypes/discovery/discover.mjs <folder>\n');
    process.exitCode = 2;
  } else {
    const result = await discover(root);
    process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
  }
}

