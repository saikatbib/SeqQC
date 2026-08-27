import assert from 'node:assert/strict';
import { mkdtemp, mkdir, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { discover } from './discover.mjs';
import { checkTaskPlatform } from '../workflow/platform-policy.mjs';

test('discovers nested sequencing files and proposes paired reads', async (context) => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'seqqc-discovery-'));
  context.after(async () => rm(root, { recursive: true, force: true }));
  await mkdir(path.join(root, 'lane 1'), { recursive: true });
  await mkdir(path.join(root, 'aligned'), { recursive: true });
  await writeFile(path.join(root, 'lane 1', 'Tumor_S1_L001_R1_001.fastq.gz'), '');
  await writeFile(path.join(root, 'lane 1', 'Tumor_S1_L001_R2_001.fastq.gz'), '');
  await writeFile(path.join(root, 'lane 1', 'Normal_R1.FQ'), '');
  await writeFile(path.join(root, 'aligned', 'sample.bam'), '');
  await writeFile(path.join(root, 'notes.txt'), 'ignored');

  const result = await discover(root);
  assert.deepEqual(result.summary, {
    supported_files: 4,
    ignored_files: 1,
    fastq_files: 3,
    bam_files: 1,
    cram_files: 0
  });
  const paired = result.groups.find((group) => group.status === 'paired_candidate');
  assert.equal(paired.proposed_sample, 'Tumor');
  assert.equal(paired.lane, 'L001');
  assert.equal(paired.r1.length, 1);
  assert.equal(paired.r2.length, 1);
  assert.ok(result.groups.some((group) => group.status === 'missing_mate'));
  assert.ok(result.groups.some((group) => group.status === 'aligned_input'));
});

test('does not silently select between duplicate mates', async (context) => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'seqqc-discovery-'));
  context.after(async () => rm(root, { recursive: true, force: true }));
  await writeFile(path.join(root, 'Sample_R1.fastq'), '');
  await writeFile(path.join(root, 'Sample.R1.fastq'), '');
  await writeFile(path.join(root, 'Sample_R2.fastq'), '');

  const result = await discover(root);
  const statuses = result.groups.map((group) => group.status);
  assert.ok(statuses.includes('ambiguous_duplicate_mate'));
  assert.ok(!statuses.includes('paired_candidate'));
});

test('gates alignment to Linux while preserving cross-platform QC', () => {
  for (const platform of ['windows', 'macos']) {
    assert.equal(checkTaskPlatform({ capability: 'alignment' }, platform).code, 'ALIGNMENT_REQUIRES_LINUX');
    assert.equal(checkTaskPlatform({ capability: 'qc' }, platform).allowed, true);
  }
  assert.equal(checkTaskPlatform({ capability: 'alignment' }, 'linux').allowed, true);
});
