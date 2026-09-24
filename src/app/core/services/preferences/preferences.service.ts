import { Injectable } from '@angular/core';
import { PREFERENCES_FILE } from '@core/ipc/bindings';
import { KeyValueStore, PREFERENCES_STORE_LOADER } from './key-value-store';

// Re-exported so the specs substituting the plugin have one import to reach for.
export { PREFERENCES_STORE_LOADER };

/**
 * `tauri-plugin-store` resolves a relative path against `BaseDirectory::AppData`: the same as
 * the config directory on Windows, not on Linux.
 */
const STORE_FILE = PREFERENCES_FILE;

/**
 * The line between the two files: `devnotes.notes.*` is the library's
 * (`LibraryPreferencesService`), everything else follows the person. `automaticBackups` stays
 * here: Rust reads it from this file before the front end boots (`backup::wanted`).
 */
export const LIBRARY_KEY_PREFIX = 'devnotes.notes.';

@Injectable({ providedIn: 'root' })
export class PreferencesService extends KeyValueStore {
  async hydrate(): Promise<void> {
    await this.open(STORE_FILE);
    this.adoptLegacyValues();
  }

  /** The library-scoped keys this file still holds from before the registry, to hand over. */
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
