import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { LibraryPreferencesService } from '@core/services/preferences/library-preferences.service';
import { DEFAULT_PRETTIER_SETTINGS } from './format.model';
import { PrettierSettingsStore } from './prettier-settings.store';

describe('PrettierSettingsStore', () => {
  beforeEach(() => TestBed.configureTestingModule({}));

  it("starts from what the library's file says", () => {
    TestBed.inject(LibraryPreferencesService).write(
      'devnotes.notes.prettier',
      JSON.stringify({ printWidth: 80 }),
    );

    expect(TestBed.inject(PrettierSettingsStore).settings()).toEqual({
      ...DEFAULT_PRETTIER_SETTINGS,
      printWidth: 80,
    });
  });

  it('applies a change at once and writes it to the library', () => {
    const store = TestBed.inject(PrettierSettingsStore);

    store.update({ quotes: 'double' });

    expect(store.settings().quotes).toBe('double');
    expect(JSON.parse(TestBed.inject(LibraryPreferencesService).read('devnotes.notes.prettier')!)).toEqual({
      ...DEFAULT_PRETTIER_SETTINGS,
      quotes: 'double',
    });
  });
});
