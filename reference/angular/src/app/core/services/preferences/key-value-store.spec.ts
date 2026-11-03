import { Injectable } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { KeyValueStore, PREFERENCES_STORE_LOADER } from './key-value-store';

/** The base is abstract; this opens whichever file it is told to. */
@Injectable({ providedIn: 'root' })
class OneFile extends KeyValueStore {
  openAt(path: string): Promise<void> {
    return this.open(path);
  }

  allKeys(): readonly string[] {
    return this.keys();
  }
}

function fakeStore(initial: [string, unknown][] = []) {
  const entries = new Map<string, unknown>(initial);
  return {
    entries: vi.fn(async () => [...entries.entries()]),
    set: vi.fn(async (key: string, value: unknown) => void entries.set(key, value)),
    delete: vi.fn(async (key: string) => entries.delete(key)),
  };
}

describe('KeyValueStore', () => {
  const load = vi.fn();
  let file: OneFile;

  beforeEach(() => {
    load.mockReset();
    TestBed.configureTestingModule({ providers: [{ provide: PREFERENCES_STORE_LOADER, useValue: load }] });
    file = TestBed.inject(OneFile);
  });

  it('opens the file it is given, saving on its own', async () => {
    load.mockResolvedValue(fakeStore([['theme', 'light']]));

    await file.openAt('libraries/a/preferences.json');

    expect(load).toHaveBeenCalledWith('libraries/a/preferences.json', { autoSave: 300 });
    expect(file.read('theme')).toBe('light');
  });

  /** Another library's file must not answer for keys the first one held. */
  it('forgets what the previous file held when it opens another', async () => {
    load.mockResolvedValueOnce(fakeStore([['marker', 'yes']])).mockResolvedValueOnce(fakeStore());

    await file.openAt('a.json');
    await file.openAt('b.json');

    expect(file.read('marker')).toBeNull();
    expect(file.allKeys()).toEqual([]);
  });

  /** A key removed is absent, not empty: a guard reading "has this key" must see no key. */
  it('removes a forgotten key from the cache and from the file', async () => {
    const store = fakeStore([['marker', 'yes']]);
    load.mockResolvedValue(store);
    await file.openAt('a.json');

    file.forget('marker');

    expect(file.read('marker')).toBeNull();
    expect(store.delete).toHaveBeenCalledWith('marker');
  });

  it('runs the session from memory when the plugin is not there', async () => {
    load.mockRejectedValue(new Error('not in Tauri'));
    await file.openAt('a.json');

    file.write('theme', 'dark');

    expect(file.read('theme')).toBe('dark');
  });
});
