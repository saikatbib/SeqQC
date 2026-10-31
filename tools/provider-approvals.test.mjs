import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { validateProviderApprovals } from './check-provider-approvals.mjs';

const baseline = JSON.parse(await readFile(new URL('../config/provider-approvals.json', import.meta.url), 'utf8'));
test('provider gate rejects enabling execution or claiming an artifact approval', () => {
  assert.deepEqual(validateProviderApprovals(baseline), { approved: 0, pending: 8 });
  assert.throws(() => validateProviderApprovals({ ...baseline, external_execution_enabled: true }));
  assert.throws(() => validateProviderApprovals({ ...baseline, approved_artifacts: [{ id: 'fastp', status: 'approved' }] }));
});
test('provider gate requires complete candidate coverage and explicit pending decisions', () => {
  assert.throws(() => validateProviderApprovals({ ...baseline, candidates: baseline.candidates.slice(1) }));
  const altered = structuredClone(baseline);
  altered.candidates[0].decision = 'approved';
  assert.throws(() => validateProviderApprovals(altered));
});
