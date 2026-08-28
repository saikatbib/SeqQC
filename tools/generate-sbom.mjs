import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { randomUUID } from 'node:crypto';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const rootPackage = JSON.parse(await readFile(path.join(root, 'package.json'), 'utf8'));
const projectVersion = rootPackage.version;
const candidateDocument = JSON.parse(await readFile(path.join(root, 'config', 'dependency-candidates.json'), 'utf8'));
const licenseSignals = new Map(candidateDocument.candidates.map((item) => [item.id, item.license_signal]));
const cargoLock = await readFile(path.join(root, 'Cargo.lock'), 'utf8');
const components = [];
for (const block of cargoLock.split('[[package]]').slice(1)) {
  const name = /^name = "([^"]+)"/m.exec(block)?.[1];
  const version = /^version = "([^"]+)"/m.exec(block)?.[1];
  if (!name || !version) continue;
  const license = licenseSignals.get(name);
  components.push({
    type: 'library',
    'bom-ref': `pkg:cargo/${encodeURIComponent(name)}@${version}`,
    name,
    version,
    ...(license ? { licenses: [{ expression: license }] } : {}),
    purl: `pkg:cargo/${encodeURIComponent(name)}@${version}`,
  });
}

const lock = await readFile(path.join(root, 'pnpm-lock.yaml'), 'utf8');
const packageBlock = lock.split(/^packages:\s*$/m)[1]?.split(/^snapshots:\s*$/m)[0] ?? '';
for (const line of packageBlock.split(/\r?\n/)) {
  const match = /^ {2}(.+):$/.exec(line);
  if (!match) continue;
  const key = match[1].replace(/^['"]|['"]$/g, '');
  const separator = key.lastIndexOf('@');
  if (separator <= 0) continue;
  const name = key.slice(0, separator);
  const version = key.slice(separator + 1);
  components.push({
    type: 'library',
    'bom-ref': `pkg:npm/${encodeURIComponent(name)}@${version}`,
    name,
    version,
    purl: `pkg:npm/${encodeURIComponent(name)}@${version}`,
  });
}

components.sort((left, right) => left['bom-ref'].localeCompare(right['bom-ref']));
const unique = components.filter((item, index) => index === 0 || item['bom-ref'] !== components[index - 1]['bom-ref']);
const bom = {
  bomFormat: 'CycloneDX',
  specVersion: '1.6',
  serialNumber: `urn:uuid:${randomUUID()}`,
  version: 1,
  metadata: {
    timestamp: new Date().toISOString(),
    component: {
      type: 'application',
      'bom-ref': `pkg:github/saikatbib/SeqQC@${projectVersion}`,
      name: 'SeqQC Studio',
      version: projectVersion,
      licenses: [{ expression: 'Apache-2.0' }],
      purl: `pkg:github/saikatbib/SeqQC@${projectVersion}`,
    },
    properties: [
      { name: 'seqqc:runtime-network-required', value: 'false' },
      { name: 'seqqc:source-lockfiles', value: 'Cargo.lock,pnpm-lock.yaml' },
    ],
  },
  components: unique,
};

const output = path.join(root, 'artifacts', 'sbom', 'seqqc.cdx.json');
await mkdir(path.dirname(output), { recursive: true });
await writeFile(output, `${JSON.stringify(bom, null, 2)}\n`, 'utf8');
process.stdout.write(`CycloneDX SBOM written with ${unique.length} locked components: ${output}\n`);
