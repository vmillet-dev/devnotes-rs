import { InjectionToken, inject } from '@angular/core';
import { load } from '@tauri-apps/plugin-store';
import type { Store, StoreOptions } from '@tauri-apps/plugin-store';

/**
 * ⚠️ A token rather than a direct call to `load`: the Angular builder bundles the modules before
 * Vitest sees them, and `vi.mock` then intercepts only half the time.
 */
type PreferencesStoreLoader = (path: string, options: StoreOptions) => Promise<Store>;

export const PREFERENCES_STORE_LOADER = new InjectionToken<PreferencesStoreLoader>(
  'PREFERENCES_STORE_LOADER',
  { providedIn: 'root', factory: () => load },
);

const AUTO_SAVE_MS = 300;

/**
 * One file of string preferences, read synchronously — a preference is read while a component
 * is constructed, and an async read would show two states in turn. Outside Tauri it degrades
 * to a memory cache. Two subclasses, one per scope: the application and the library.
 */
export abstract class KeyValueStore {
  private readonly load = inject(PREFERENCES_STORE_LOADER);
  private readonly cache = new Map<string, string>();
  private store: Store | null = null;

  /** Before the first read, which would otherwise answer `null`. */
  protected async open(path: string): Promise<void> {
    this.cache.clear();
    this.store = null;

    try {
      const store = await this.load(path, { autoSave: AUTO_SAVE_MS });
      for (const [key, value] of await store.entries<unknown>()) {
        if (typeof value === 'string') {
          this.cache.set(key, value);
        }
      }
      this.store = store;
    } catch {
      // Plugin unavailable: the memory cache runs the session, which will not survive
      // a restart.
    }
  }

  read(key: string): string | null {
    return this.cache.get(key) ?? null;
  }

  write(key: string, value: string): void {
    this.cache.set(key, value);
    void this.store?.set(key, value).catch(() => undefined);
  }

  /** Removed, not emptied: a guard asking "is this key here" would take `''` for an answer. */
  forget(key: string): void {
    this.cache.delete(key);
    void this.store?.delete(key).catch(() => undefined);
  }

  /** Every key this file holds, for the one caller that has to move some of them. */
  protected keys(): readonly string[] {
    return [...this.cache.keys()];
  }
}
