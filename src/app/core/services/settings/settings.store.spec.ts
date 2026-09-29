import { TestBed } from '@angular/core/testing';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { PreferencesService } from '@core/services/preferences/preferences.service';
import { SETTINGS_KEYS } from './app-settings.model';
import { SettingsStore } from './settings.store';

/** What the WebView would get from the OS; jsdom supplies one, always false. */
function stubSystemTheme(prefersDark: boolean): void {
  vi.stubGlobal('matchMedia', (query: string) => ({
    matches: prefersDark,
    media: query,
    addEventListener: () => undefined,
    removeEventListener: () => undefined,
  }));
}

describe('SettingsStore', () => {
  /** Mirrors the initializer: build it, then read back what was stored. */
  function createStore(): SettingsStore {
    const store = TestBed.inject(SettingsStore);
    store.restore();
    return store;
  }

  function preferences(): PreferencesService {
    return TestBed.inject(PreferencesService);
  }

  beforeEach(() => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({});
    document.documentElement.removeAttribute('data-theme');
    document.documentElement.removeAttribute('data-density');
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('starts on the documented defaults', () => {
    const store = createStore();

    expect(store.locale()).toBe('system');
    expect(store.theme()).toBe('system');
    expect(store.density()).toBe('comfortable');
    expect(store.closeToTray()).toBe(true);
    expect(store.minimizeToTray()).toBe(false);
    expect(store.paletteShortcut()).toBe('Ctrl+Alt+P');
    expect(store.showPinnedFirst()).toBe(true);
    expect(store.copyConfirmation()).toBe(true);
  });

  it('restores what a previous session stored', () => {
    preferences().write(SETTINGS_KEYS.theme, 'light');
    preferences().write(SETTINGS_KEYS.density, 'compact');
    preferences().write(SETTINGS_KEYS.closeToTray, 'false');
    preferences().write(SETTINGS_KEYS.paletteShortcut, 'Ctrl+Shift+K');

    const store = createStore();

    expect(store.theme()).toBe('light');
    expect(store.density()).toBe('compact');
    expect(store.closeToTray()).toBe(false);
    expect(store.paletteShortcut()).toBe('Ctrl+Shift+K');
  });

  it('ignores a stored value that is no longer a valid choice', () => {
    preferences().write(SETTINGS_KEYS.theme, 'solarized');

    expect(createStore().theme()).toBe('system');
  });

  /** Remembered with the application; a hand-edited file loses its bad entries, not the list. */
  it('reads the recent tools back, entry by entry', () => {
    preferences().write(
      SETTINGS_KEYS.recentTools,
      JSON.stringify([{ id: 'hash', at: '2026-09-29T10:00:00.000Z' }, { id: 42 }, 'slug']),
    );

    expect(createStore().recentTools()).toEqual([{ id: 'hash', at: '2026-09-29T10:00:00.000Z' }]);
  });

  it('starts with no recent tool when the file holds no list', () => {
    preferences().write(SETTINGS_KEYS.recentTools, '{not json');

    expect(createStore().recentTools()).toEqual([]);
  });

  it('keeps the default of a flag nothing has stored, rather than reading it as false', () => {
    expect(createStore().closeToTray()).toBe(true);
  });

  it('persists every change it accepts', () => {
    const store = createStore();

    store.setTheme('dark');
    store.setDensity('compact');
    store.setCopyConfirmation(false);

    expect(preferences().read(SETTINGS_KEYS.theme)).toBe('dark');
    expect(preferences().read(SETTINGS_KEYS.density)).toBe('compact');
    expect(preferences().read(SETTINGS_KEYS.copyConfirmation)).toBe('false');
  });

  it('refuses an empty shortcut, which would leave the palette without a call', () => {
    const store = createStore();

    store.setPaletteShortcut('   ');

    expect(store.paletteShortcut()).toBe('Ctrl+Alt+P');
  });

  it('resolves the system theme from what the OS asks for', () => {
    stubSystemTheme(true);

    expect(createStore().resolvedTheme()).toBe('dark');
  });

  it('follows an explicit choice over the system preference', () => {
    stubSystemTheme(true);
    const store = createStore();

    store.setTheme('light');

    expect(store.resolvedTheme()).toBe('light');
  });

  it('stamps the resolved theme and the density on the document', () => {
    const store = createStore();
    store.setTheme('light');
    store.setDensity('compact');
    TestBed.tick();

    expect(document.documentElement.dataset['theme']).toBe('light');
    expect(document.documentElement.dataset['density']).toBe('compact');
  });

  it('survives an environment without matchMedia', () => {
    vi.stubGlobal('matchMedia', undefined);

    expect(() => createStore()).not.toThrow();
  });
});
