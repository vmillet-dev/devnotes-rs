import { describe, expect, it } from 'vitest';
import { AUTOMATIC_BACKUPS_KEY } from '@core/ipc/bindings';
import { DEFAULT_SHORTCUTS } from '@core/services/shortcuts/shortcut.model';
import {
  AppSettings,
  DEFAULT_SETTINGS,
  DENSITIES,
  LOCALE_CHOICES,
  SETTINGS_KEYS,
  THEME_CHOICES,
} from './app-settings.model';

describe('app settings model', () => {
  /**
   * ⚠️ Derived rather than hand-written, and this is what makes "a setting is one line"
   * true: a field added to `AppSettings` gets its key for free, and cannot get a
   * hand-typed one that drifts from the field name.
   */
  it('gives every setting a key, and every key its own field name', () => {
    const fields = Object.keys(DEFAULT_SETTINGS) as (keyof AppSettings)[];

    expect(Object.keys(SETTINGS_KEYS).sort()).toEqual([...fields].sort());
    for (const field of fields) {
      expect(SETTINGS_KEYS[field]).toBe(`devnotes.${field}`);
    }
  });

  /** Rust reads this one out of the file itself, before the front end has booted. */
  it('writes the backups switch under the key Rust reads it by', () => {
    expect(SETTINGS_KEYS.automaticBackups).toBe(AUTOMATIC_BACKUPS_KEY);
  });

  it('namespaces every key, since the store is shared with everything else', () => {
    expect(Object.values(SETTINGS_KEYS).every((key) => key.startsWith('devnotes.'))).toBe(true);
    expect(new Set(Object.values(SETTINGS_KEYS)).size).toBe(Object.keys(SETTINGS_KEYS).length);
  });

  describe('the defaults', () => {
    it('offers "system" as a choice for what the OS can answer, and nothing else', () => {
      expect(LOCALE_CHOICES).toEqual(['system', 'fr', 'en']);
      expect(THEME_CHOICES).toEqual(['system', 'dark', 'light']);
      // ⚠️ Density is spacing only: a density that shrank the type would be a zoom.
      expect(DENSITIES).toEqual(['comfortable', 'compact']);
    });

    it('defers to the machine wherever the machine has an opinion', () => {
      expect(DEFAULT_SETTINGS.locale).toBe('system');
      expect(DEFAULT_SETTINGS.theme).toBe('system');
    });

    /** ⚠️ `true`, and `desktop.rs` carries the same default on the native side. */
    it('closes to the tray rather than quitting', () => {
      expect(DEFAULT_SETTINGS.closeToTray).toBe(true);
      expect(DEFAULT_SETTINGS.minimizeToTray).toBe(false);
      expect(DEFAULT_SETTINGS.startWithSystem).toBe(false);
    });

    /**
     * ⚠️ On, and Rust agrees by reading anything that is not a plain "false" as on: a
     * safety net nobody asked to remove stays.
     */
    it('copies the library unless somebody says otherwise', () => {
      expect(DEFAULT_SETTINGS.automaticBackups).toBe(true);
    });

    /** The other half of the mirror lives in `ShortcutBindings::defaults()`. */
    it('takes the palette shortcut from the one table that declares it', () => {
      expect(DEFAULT_SETTINGS.paletteShortcut).toBe(DEFAULT_SHORTCUTS.palette);
    });

    /**
     * ⚠️ A version and not a boolean: remembering "no" would silence the release after
     * it too, and the empty string is what "nothing is silenced" looks like.
     */
    it('silences no update until one is named', () => {
      expect(DEFAULT_SETTINGS.skippedUpdate).toBe('');
      expect(DEFAULT_SETTINGS.updateNotifications).toBe(true);
    });

    it('is complete: every declared key has a value to fall back on', () => {
      for (const value of Object.values(DEFAULT_SETTINGS)) {
        expect(value).toBeDefined();
      }
    });
  });
});
