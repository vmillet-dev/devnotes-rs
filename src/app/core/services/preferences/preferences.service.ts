import { Injectable } from '@angular/core';
import { PREFERENCES_FILE } from '@core/ipc/bindings';
import { KeyValueStore, PREFERENCES_STORE_LOADER } from './key-value-store';

// Re-exported from where it was, so the dozen specs that substitute the plugin keep one
// import to reach for.
export { PREFERENCES_STORE_LOADER };

/**
 * ⚠️ `app_data_dir()`, not `app_config_dir()`: `tauri-plugin-store` resolves a relative
 * path against `BaseDirectory::AppData`. The two are the same directory on Windows and
 * only Linux splits them.
 */
const STORE_FILE = PREFERENCES_FILE;

/**
 * ⚠️ Everything a key can be **except** what belongs to one library's notes. Which
 * library is open changes nothing here: the theme, the language, the keys, the tray and
 * the window geometry follow the person, not the corpus.
 *
 * ⚠️ The line is one prefix. `devnotes.notes.*` is the library's — see
 * `LibraryPreferencesService` — and everything else is this file's. `automaticBackups`
 * stays here deliberately: "copy my libraries at launch" is a habit rather than a
 * property of one corpus, and it is the one key Rust reads out of this file before the
 * front end has booted (`backup::wanted`).
 */
export const LIBRARY_KEY_PREFIX = 'devnotes.notes.';

@Injectable({ providedIn: 'root' })
export class PreferencesService extends KeyValueStore {
  async hydrate(): Promise<void> {
    await this.open(STORE_FILE);
    this.adoptLegacyValues();
  }

  /**
   * The library-scoped keys this file used to hold, so they can be moved into the library
   * that owns them.
   *
   * ⚠️ Every install before the registry kept both scopes in one file. Without this, the
   * first launch after the upgrade reads no samples marker and re-seeds a library that is
   * full, and forgets which view each space was left on.
   */
  libraryScoped(): readonly [string, string][] {
    return this.keys()
      .filter((key) => key.startsWith(LIBRARY_KEY_PREFIX))
      .map((key) => [key, this.read(key) ?? ''] as [string, string]);
  }

  private adoptLegacyValues(): void {
    const adopted: string[] = [];
    try {
      for (let index = 0; index < localStorage.length; index++) {
        const key = localStorage.key(index);
        // Our keys only: dumping everything would pollute the preferences file for good.
        if (!key?.startsWith('devnotes.') || this.read(key) !== null) continue;

        const value = localStorage.getItem(key);
        if (value === null) continue;

        this.write(key, value);
        adopted.push(key);
      }
    } catch {
      // `localStorage` throws in private browsing.
      return;
    }

    // Only what was adopted: a `clear()` would also take the keys this loop refused.
    for (const key of adopted) {
      try {
        localStorage.removeItem(key);
      } catch {
        // Harmless: the adoption already happened on the file side.
      }
    }
  }
}
