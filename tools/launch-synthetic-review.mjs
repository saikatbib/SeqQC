import { spawn } from 'node:child_process';
import { mkdtemp } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';

if (!process.argv[2]) throw new Error('Pass an explicitly built qualification executable');
const binary = path.resolve(process.argv[2]);
const parent = await mkdtemp(path.join(os.tmpdir(), 'seqqc-live-review-'));
const env = { ...process.env, SEQQC_QA_ROOT: path.join(parent, 'synthetic'), SEQQC_QA_SCENE: 'inputs', SEQQC_QA_THEME: 'light' };
delete env.SEQQC_QA_SCREENSHOT;
delete env.EFRAME_SCREENSHOT_TO;
const child = spawn(binary, [], { env, windowsHide: true, detached: true, stdio: 'ignore' });
child.once('error', error => { throw error; });
child.once('spawn', () => { process.stdout.write(`Started isolated synthetic review, PID ${child.pid}\n`); child.unref(); });
