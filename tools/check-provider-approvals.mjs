import { readFile } from 'node:fs/promises';

export function validateProviderApprovals(document) {
  if (document.schema_version !== '1.0.0') throw new Error('Unsupported provider approval schema');
  if (document.external_execution_enabled !== false || !Array.isArray(document.approved_artifacts) || document.approved_artifacts.length !== 0) {
    throw new Error('External execution is unavailable until signed policy verification, adapters, and independent artifact approval are implemented');
  }
  const expected = new Set(['samtools-htslib', 'fastp', 'bwa-mem2', 'star', 'mosdepth', 'picard', 'rnaseqc', 'verifybamid2']);
  for (const candidate of document.candidates ?? []) {
    if (!expected.delete(candidate.id)) throw new Error(`Unexpected or duplicate provider: ${candidate.id}`);
    if (candidate.decision !== 'pending_independent_review' || !Array.isArray(candidate.blockers) || candidate.blockers.length === 0) throw new Error(`Provider approval is unsupported or lacks blockers: ${candidate.id}`);
    if (candidate.upstream_reported_sha256 != null && !/^[0-9a-f]{64}$/.test(candidate.upstream_reported_sha256)) throw new Error(`Invalid upstream digest: ${candidate.id}`);
    if (candidate.digest_independently_verified === true) throw new Error(`No independent artifact verification is recorded for ${candidate.id}`);
  }
  if (expected.size) throw new Error(`Missing provider reviews: ${[...expected].join(', ')}`);
  return { approved: 0, pending: document.candidates.length };
}

export async function checkProviderApprovals() {
  return validateProviderApprovals(JSON.parse(await readFile(new URL('../config/provider-approvals.json', import.meta.url), 'utf8')));
}
