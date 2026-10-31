// Native framebuffer captures of an explicitly isolated synthetic build.
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdir, mkdtemp, readFile, writeFile, access } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';

const options = Object.fromEntries(process.argv.slice(2).reduce((pairs, value, index, args) => {
  if (index % 2 === 0) pairs.push([value.replace(/^--/, ''), args[index + 1]]);
  return pairs;
}, []));
if (!options.binary || !options.output) throw new Error('Usage: node tools/qualify-native.mjs --binary <qualification executable> --output <new evidence directory> [--scenes locked,inputs]');
const binary = path.resolve(options.binary);
const output = path.resolve(options.output);
await mkdir(output, { recursive: true });
const results = { schema_version: '1.0.0', captured_at: new Date().toISOString(), platform: process.platform, architecture: process.arch, binary_sha256: createHash('sha256').update(await readFile(binary)).digest('hex'), scope: 'Synthetic native-framebuffer screenshots; no human usability or screen-reader certification', snapshots: [] };
const scenes = (options.scenes ?? 'locked,overview,projects,inputs,readiness,recovery,large').split(',');
if (scenes.some(scene => !['locked', 'overview', 'projects', 'inputs', 'readiness', 'recovery', 'large'].includes(scene))) throw new Error('Unknown synthetic scene');

function run(command, args, env) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { env, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
    let log = '';
    child.stdout.on('data', data => { log = (log + data).slice(-12000); });
    child.stderr.on('data', data => { log = (log + data).slice(-12000); });
    const timer = setTimeout(() => { child.kill(); reject(new Error(`Native capture exceeded 60 seconds: ${log}`)); }, 60000);
    child.once('error', error => { clearTimeout(timer); reject(error); });
    child.once('close', code => { clearTimeout(timer); if (code === 0) resolve(log); else reject(new Error(`Native capture exited ${code}: ${log}`)); });
  });
}

for (const theme of ['night', 'light', 'contrast']) {
  for (const zoom of [1, 2]) {
    for (const scene of scenes) {
      const name = `${theme}-${zoom}x-${scene}`;
      const png = path.join(output, `${name}.png`);
      try { await access(png); throw new Error(`Refusing stale screenshot: ${png}`); } catch (error) { if (error.code !== 'ENOENT') throw error; }
      const fixtureParent = await mkdtemp(path.join(os.tmpdir(), 'seqqc-qualification-'));
      const env = { ...process.env, SEQQC_QA_ROOT: path.join(fixtureParent, 'synthetic'), SEQQC_QA_THEME: theme, SEQQC_QA_SCENE: scene, SEQQC_QA_ZOOM: String(zoom), SEQQC_QA_SCREENSHOT: png };
      const trace = path.join(output, `${name}.strace.txt`);
      const command = process.platform === 'linux' ? 'strace' : binary;
      const args = process.platform === 'linux' ? ['-f', '-qq', '-e', 'trace=network,process', '-o', trace, '--', binary] : [];
      try {
        await run(command, args, env);
        const bytes = await readFile(png);
        if (bytes.subarray(0, 8).toString('hex') !== '89504e470d0a1a0a') throw new Error('Invalid PNG signature');
        const width = bytes.readUInt32BE(16);
        const height = bytes.readUInt32BE(20);
        if (width < 800 || height < 600) throw new Error(`Unexpected native frame size ${width}x${height}`);
        if (process.platform === 'linux') {
          const calls = await readFile(trace, 'utf8');
          if (!calls.includes('execve(')) throw new Error('Empty or incomplete process trace');
          if (/socket\(AF_INET6?\b|sa_family=AF_INET6?\b/.test(calls)) throw new Error('Internet-family network operation observed');
        }
        results.snapshots.push({ name, status: 'captured', width, height, sha256: createHash('sha256').update(bytes).digest('hex'), network_scope: process.platform === 'linux' ? 'No internet-family syscall observed during this scene' : 'Not measured by framebuffer capture' });
        process.stdout.write(`Captured ${name}: ${width}x${height}\n`);
      } catch (error) {
        results.snapshots.push({ name, status: 'failed', error: error.message });
        await writeFile(path.join(output, 'results.json'), `${JSON.stringify(results, null, 2)}\n`);
        throw error;
      }
      await writeFile(path.join(output, 'results.json'), `${JSON.stringify(results, null, 2)}\n`);
    }
  }
}
