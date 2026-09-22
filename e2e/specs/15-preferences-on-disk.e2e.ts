import { expect } from '@wdio/globals';
import { existsSync, readFileSync } from 'node:fs';

import { canvas } from '../pageobjects/canvas.page.js';
import { fileMenu, settings, titlebar } from '../pageobjects/titlebar.page.js';
import { eventually } from '../support/app.js';
import { preferencesPath } from '../support/profile.js';

/**
 * ⚠️ Nothing in the suite can restart the application, and `tauri-plugin-store` holds its
 * values in that same process — so a reloaded page reads the in-memory map, and a
 * preference could never touch the disk with every assertion in `12-preferences` still
 * passing. The file is read from Node instead, outside the application entirely.
 *
 * ⚠️ The data directory, next to the database. Windows cannot tell it from the config one,
 * so this file only says anything about the location on Linux. See `support/profile.ts`.
 */
describe('Preferences reach the disk', () => {
  function stored(): Record<string, unknown> {
    const path = preferencesPath();
    if (!existsSync(path)) {
      throw new Error(`no preferences file at ${path}`);
    }
    return JSON.parse(readFileSync(path, 'utf8')) as Record<string, unknown>;
  }

  /**
   * ⚠️ The file *is* the condition, and it is polled from Node: the page can only say the
   * in-memory map moved, which is the one thing this file exists not to trust. The fixed
   * wait it replaces was longer than the plugin's `autoSave` debounce, and paid that worst
   * case on every assertion.
   */
  function settled(key: string, want: unknown): Promise<Record<string, unknown>> {
    return eventually(
      async () => (existsSync(preferencesPath()) ? stored() : {}),
      (file) => file[key] === want,
      `${key} to reach the preferences file as ${String(want)}`,
    );
  }

  before(canvas.open);

  it('writes the file at all, next to the database', async () => {
    // A spec file establishes its own preconditions: one profile serves the whole run.
    await fileMenu.openPreferences();
    await settings.setTheme('dark');
    await settings.close();
    await settled('devnotes.theme', 'dark');

    expect(existsSync(preferencesPath())).toBe(true);
  });

  it('writes one key per setting, never one serialised object', async () => {
    await fileMenu.openPreferences();
    await settings.setDensity('comfortable');
    await settings.close();

    const file = await settled('devnotes.density', 'comfortable');
    // ⚠️ One key per setting: a blob under one key would make a half-written file lose
    // every setting at once.
    expect(file['devnotes.theme']).toBe('dark');
    expect(file['devnotes.density']).toBe('comfortable');
  });

  it('rewrites the key as the value is changed, with no confirmation step', async () => {
    await fileMenu.openPreferences();
    await settings.setTheme('light');
    await settings.close();

    // There is no OK anywhere in the panel: closing it is not what saves.
    expect((await settled('devnotes.theme', 'light'))['devnotes.theme']).toBe('light');
  });

  it('stores the locale the titlebar switch chose, like the panel does', async () => {
    await titlebar.setLocale('en');

    expect((await settled('devnotes.locale', 'en'))['devnotes.locale']).toBe('en');
    expect(await titlebar.activeLocale()).toBe('en');
  });

  it('holds values as strings, one codec per setting', async () => {
    const file = stored();
    // ⚠️ `PreferencesService` caches `string` and nothing else — a boolean written as a
    // boolean would be dropped by `hydrate()` on the next launch.
    const notStrings = Object.entries(file)
      .filter(([, value]) => typeof value !== 'string')
      .map(([key]) => key);
    expect(notStrings).toEqual([]);
  });
});
