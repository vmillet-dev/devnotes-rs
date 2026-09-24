import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { PREFERENCES_FILE } from '@core/ipc/bindings';
import { PREFERENCES_STORE_LOADER } from './key-value-store';
import { LibraryPreferencesService } from './library-preferences.service';
import { PreferencesService } from './preferences.service';

function fakeStore(initial: [string, unknown][] = []) {
  const entries = new Map<string, unknown>(initial);
  return {
    entries: vi.fn(async () => [...entries.entries()]),
    set: vi.fn(async (key: string, value: unknown) => void entries.set(key, value)),
    delete: vi.fn(async (key: string) => entries.delete(key)),
  };
}

describe('LibraryPreferencesService', () => {
  const files = new Map<string, ReturnType<typeof fakeStore>>();
  let application: PreferencesService;
  let library: LibraryPreferencesService;

  beforeEach(async () => {
    files.clear();
    localStorage.clear();
    TestBed.configureTestingModule({
      providers: [
        {
          provide: PREFERENCES_STORE_LOADER,
          useValue: async (path: string) => {
            if (!files.has(path)) files.set(path, fakeStore());
            return files.get(path);
          },
        },
      ],
    });
    application = TestBed.inject(PreferencesService);
    library = TestBed.inject(LibraryPreferencesService);
  });

  async function upgradeFrom(entries: [string, string][]): Promise<void> {
    files.set(PREFERENCES_FILE, fakeStore(entries));
    await application.hydrate();
  }

  it("opens the library's own file, beside its database", async () => {
    await library.hydrate('libraries/a');

    expect(files.has(`libraries/a/${PREFERENCES_FILE}`)).toBe(true);
  });

  /** ⚠️ Every install before the registry kept both scopes in the application's file. */
  it("moves the library's keys out of the application's file, once", async () => {
    await upgradeFrom([
      ['devnotes.notes.samplesSeeded', 'true'],
      ['devnotes.theme', 'dark'],
    ]);

    await library.hydrate('libraries/a');

    expect(library.read('devnotes.notes.samplesSeeded')).toBe('true');
    expect(application.read('devnotes.notes.samplesSeeded')).toBeNull();
    expect(application.read('devnotes.theme')).toBe('dark');
  });

  it('keeps what the library already holds over what the application carried', async () => {
    files.set(`libraries/a/${PREFERENCES_FILE}`, fakeStore([['devnotes.notes.view.s-1', 'board']]));
    await upgradeFrom([['devnotes.notes.view.s-1', 'date']]);

    await library.hydrate('libraries/a');

    expect(library.read('devnotes.notes.view.s-1')).toBe('board');
  });

  /** ⚠️ A second library must not inherit the first one's samples marker. */
  it('leaves nothing for the next library to adopt', async () => {
    await upgradeFrom([['devnotes.notes.samplesSeeded', 'true']]);
    await library.hydrate('libraries/a');

    await library.hydrate('libraries/b');

    expect(library.read('devnotes.notes.samplesSeeded')).toBeNull();
  });
});
