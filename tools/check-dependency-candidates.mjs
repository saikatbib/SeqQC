import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

export async function checkDependencyCandidates() {
  const file = path.join(repositoryRoot, 'config', 'dependency-candidates.json');
  const document = JSON.parse(await readFile(file, 'utf8'));
  if (document.schema_version !== '1.0.0') throw new Error('Unsupported dependency candidate schema version.');
  const ids = new Set();
  for (const candidate of document.candidates ?? []) {
    for (const field of ['id', 'role', 'decision', 'license_signal', 'source']) {
      if (!candidate[field]) throw new Error(`Dependency candidate is missing ${field}: ${candidate.id ?? '<unknown>'}`);
    }
    if (ids.has(candidate.id)) throw new Error(`Duplicate dependency candidate: ${candidate.id}`);
    ids.add(candidate.id);
    if (!Array.isArray(candidate.platforms) || candidate.platforms.length === 0) throw new Error(`No platforms for ${candidate.id}`);
    if (candidate.runtime_network_required !== false) throw new Error(`Runtime network dependency is forbidden: ${candidate.id}`);
    if (candidate.role.includes('aligner') && candidate.platforms.some((platform) => platform !== 'linux')) {
      throw new Error(`Initial aligner candidate must be Linux-only: ${candidate.id}`);
    }
    if (!candidate.source.startsWith('https://')) throw new Error(`Dependency source must be an HTTPS provenance URL: ${candidate.id}`);
  }
  return { candidates: ids.size };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const result = await checkDependencyCandidates();
  process.stdout.write(`Dependency candidate check passed: ${result.candidates} records.\n`);
}

