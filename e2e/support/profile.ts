import { existsSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const IDENTIFIER = 'com.devnotes.app.e2e';

/**
 * ⚠️ `app_data_dir()` is `data_dir()/identifier`, and the e2e build carries an identifier
 * of its own — which is what keeps this wipe off the library being dogfooded.
 */
function e2eDataDir(): string {
  if (process.platform === 'win32') {
    return join(process.env['APPDATA'] ?? '', IDENTIFIER);
  }
  const xdg = process.env['XDG_DATA_HOME'];
  return xdg ? join(xdg, IDENTIFIER) : join(process.env['HOME'] ?? '', '.local/share', IDENTIFIER);
}

/**
 * A second directory, and on Linux it is not the first one:
 * `tauri-plugin-window-state` writes under `app_config_dir()` while everything else
 * writes under `app_data_dir()`. Windows cannot tell the two apart, so wiping only the
 * data directory looks complete there and leaves the geometry behind on Linux.
 */
function e2eConfigDir(): string {
  if (process.platform === 'win32') {
    return join(process.env['APPDATA'] ?? '', IDENTIFIER);
  }
  const xdg = process.env['XDG_CONFIG_HOME'];
  return xdg ? join(xdg, IDENTIFIER) : join(process.env['HOME'] ?? '', '.config', IDENTIFIER);
}

/**
 * Under the data directory, not the config one: `tauri-plugin-store` resolves a
 * relative path against `BaseDirectory::AppData`. Worth spelling out because a test
 * looking in the wrong one still passes on Windows, where the two are the same path.
 */
export function preferencesPath(): string {
  return join(e2eDataDir(), 'preferences.json');
}

/**
 * Where the open library's files sit, read out of the registry: libraries live under
 * `libraries/<id>/`, and a helper that guessed would read the wrong `vault.json` the moment a
 * spec switches library.
 */
export function openLibraryDir(): string {
  const profile = e2eDataDir();
  try {
    const registry = JSON.parse(readFileSync(join(profile, 'libraries.json'), 'utf8')) as {
      libraries?: { id: string; directory: string }[];
      open?: string | null;
    };
    const entries = registry.libraries ?? [];
    const entry = entries.find((each) => each.id === registry.open) ?? entries[0];

    return entry ? join(profile, ...entry.directory.split('/')) : profile;
  } catch {
    // No registry yet: the very first launch has not written one.
    return profile;
  }
}

/** The key file, beside the database — what a passphrase change rewrites. */
export function vaultPath(): string {
  return join(openLibraryDir(), 'vault.json');
}

/**
 * Where `homeSpaceId()` records the seeded space. Outside the profile on purpose: it is
 * the harness's own note, not the application's state.
 */
export function homeSpaceMarker(): string {
  return join(tmpdir(), 'devnotes-e2e-home-space');
}

/**
 * ⚠️ Called from `npm run test:e2e`, before WebdriverIO starts — not from a hook. The
 * embedded provider spawns the application from its own `onPrepare`, and nothing orders
 * that against the config's hooks: a wipe from inside wdio meets a locked database.
 */
export function resetProfile(): void {
  // Both, and `new Set` because on Windows they are the same path.
  const directories = [...new Set([e2eDataDir(), e2eConfigDir()])];

  for (const directory of directories) {
    rmSync(directory, { recursive: true, force: true });
  }
  rmSync(homeSpaceMarker(), { force: true });

  // `force` covers "it was not there", the ordinary case, but it also swallows the one
  // that matters: a previous run still holding the database open. The wipe then does
  // nothing, the suite starts on the corpus the last run left, and every failure after
  // it blames the wrong thing.
  const survivor = directories.find((directory) => existsSync(directory));
  if (survivor) {
    throw new Error(
      `the e2e profile at ${survivor} could not be wiped — an application from a previous run is probably still holding it open`,
    );
  }
}
