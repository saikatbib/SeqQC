import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

async function read(relativePath) {
  return readFile(path.join(repositoryRoot, relativePath), 'utf8');
}

function requireCondition(condition, message) {
  if (!condition) throw new Error(message);
}

export async function checkDesktopPolicy() {
  const tauriConfig = JSON.parse(await read('apps/desktop/src-tauri/tauri.conf.json'));
  const capability = JSON.parse(await read('apps/desktop/src-tauri/capabilities/desktop-base.json'));
  const cargoManifest = await read('apps/desktop/src-tauri/Cargo.toml');
  const frontendManifest = JSON.parse(await read('apps/desktop/package.json'));
  const frontendSource = [
    await read('apps/desktop/src/App.tsx'),
    await read('apps/desktop/src/main.tsx'),
    await read('apps/desktop/src/styles.css'),
  ].join('\n');

  const security = tauriConfig.app?.security;
  requireCondition(security?.csp?.includes("connect-src 'none'"), "Desktop CSP must deny every network connection.");
  requireCondition(security?.assetProtocol?.enable === false, 'Tauri asset protocol must remain disabled until explicitly required.');
  requireCondition(tauriConfig.app?.withGlobalTauri === false, 'Global frontend Tauri injection must be disabled.');
  requireCondition(!('devUrl' in (tauriConfig.build ?? {})), 'Desktop configuration must not declare a local or remote dev URL.');
  requireCondition(tauriConfig.bundle?.createUpdaterArtifacts === false, 'Updater artifacts must remain disabled.');
  requireCondition(
    JSON.stringify(security?.capabilities) === JSON.stringify(['desktop-base']),
    'Only the reviewed desktop-base capability may be active.',
  );
  requireCondition(
    JSON.stringify(capability.permissions) === JSON.stringify(['core:default', 'dialog:allow-open']),
    'Desktop capability permissions expanded beyond the reviewed core and open-dialog permissions.',
  );

  const forbiddenRustPlugins = ['tauri-plugin-http', 'tauri-plugin-updater', 'tauri-plugin-shell', 'tauri-plugin-fs'];
  for (const plugin of forbiddenRustPlugins) {
    requireCondition(!cargoManifest.includes(plugin), `Forbidden Rust plugin declared: ${plugin}`);
  }
  const frontendDependencies = {
    ...(frontendManifest.dependencies ?? {}),
    ...(frontendManifest.devDependencies ?? {}),
  };
  const reviewedFrontendPlugins = new Set(['@tauri-apps/plugin-dialog']);
  for (const dependency of Object.keys(frontendDependencies)) {
    requireCondition(
      !dependency.startsWith('@tauri-apps/plugin-') || reviewedFrontendPlugins.has(dependency),
      `Unreviewed frontend plugin declared: ${dependency}`,
    );
  }
  requireCondition(
    frontendManifest.dependencies?.['@tauri-apps/plugin-dialog'] === '2.7.2',
    'The reviewed dialog plugin must remain exact-pinned to 2.7.2.',
  );
  requireCondition(!/https?:\/\//i.test(frontendSource), 'Frontend source must not contain a network URL.');

  return {
    capabilities: capability.permissions.length,
    forbiddenPlugins: forbiddenRustPlugins.length,
  };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const result = await checkDesktopPolicy();
  process.stdout.write(
    `Desktop offline policy check passed: ${result.capabilities} capability permission, ${result.forbiddenPlugins} forbidden plugin families absent.\n`,
  );
}
