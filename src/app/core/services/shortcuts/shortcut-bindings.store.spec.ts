import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { PreferencesService } from '@core/services/preferences/preferences.service';
import { SettingsStore } from '@core/services/settings/settings.store';
import { ShortcutBindingsStore } from './shortcut-bindings.store';
import { GLOBAL_ACTIONS, Rebindable } from './shortcut.model';

const PALETTE = GLOBAL_ACTIONS[0];
const COPY: Rebindable = { id: 'canvas.copy', labelKey: 'shortcuts.canvas.copy', fallback: 'C' };
const PIN: Rebindable = { id: 'canvas.pin', labelKey: 'shortcuts.canvas.pin', fallback: 'P' };
const EVERY = [...GLOBAL_ACTIONS, COPY, PIN];

describe('ShortcutBindingsStore', () => {
  let store: ShortcutBindingsStore;
  let preferences: PreferencesService;
  let settings: SettingsStore;

  beforeEach(() => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({});
    store = TestBed.inject(ShortcutBindingsStore);
    preferences = TestBed.inject(PreferencesService);
    settings = TestBed.inject(SettingsStore);
  });

  it('answers the accelerator an action ships with until one is stored', () => {
    expect(store.binding(COPY)).toBe('C');
    expect(store.isDefault(COPY)).toBe(true);

    store.rebind(COPY, 'Y', EVERY);

    expect(store.binding(COPY)).toBe('Y');
    expect(store.isDefault(COPY)).toBe(false);
  });

  /**
   * Two storage paths. A canvas key is one preference of its own; the three global
   * ones live in `AppSettings`, because the native command takes them as a block.
   */
  it('writes a canvas key to its own preference and a global one to the settings', () => {
    store.rebind(COPY, 'Y', EVERY);
    store.rebind(PALETTE, 'Ctrl+Shift+K', EVERY);

    expect(preferences.read('devnotes.shortcut.canvas.copy')).toBe('Y');
    expect(settings.paletteShortcut()).toBe('Ctrl+Shift+K');
  });

  it('refuses a keystroke another action already answers to, and names it', () => {
    const refused = store.rebind(PIN, 'C', EVERY);

    expect(refused).toEqual({ kind: 'taken', by: COPY });
    expect(store.binding(PIN)).toBe('P');
  });

  /** Across both halves, or a key could be taken twice between them. */
  it('sees a global key colliding with a canvas one', () => {
    store.rebind(COPY, 'Ctrl+B', EVERY);

    expect(store.rebind(PALETTE, 'Ctrl+B', EVERY)).toEqual({ kind: 'taken', by: COPY });
  });

  /** A global shortcut with no modifier would swallow that key machine-wide. */
  it('refuses a bare key on a global action but takes one on a canvas action', () => {
    expect(store.rebind(PALETTE, 'K', EVERY)).toEqual({ kind: 'illegal' });
    expect(store.rebind(COPY, 'K', EVERY)).toBeNull();
  });

  it('refuses the keys the grid navigates with, on either half', () => {
    expect(store.rebind(COPY, 'ArrowUp', EVERY)).toEqual({ kind: 'illegal' });
    expect(store.rebind(COPY, 'Escape', EVERY)).toEqual({ kind: 'illegal' });
    expect(store.rebind(COPY, 'Tab', EVERY)).toEqual({ kind: 'illegal' });
  });

  /** Forgotten rather than written back: a default is free to move between versions. */
  it('forgets an override rather than storing the default over it', () => {
    store.rebind(COPY, 'Y', EVERY);

    store.reset(COPY);

    expect(preferences.read('devnotes.shortcut.canvas.copy')).toBeNull();
    expect(store.binding(COPY)).toBe('C');
  });

  it('puts a global action back to what the native side ships', () => {
    store.rebind(PALETTE, 'Ctrl+Shift+K', EVERY);

    store.reset(PALETTE);

    expect(settings.paletteShortcut()).toBe(PALETTE.fallback);
  });

  /** A stored value the current build cannot read is a stale file, not a binding. */
  it('falls back to the default when what is stored is not a legal keystroke', () => {
    preferences.write('devnotes.shortcut.canvas.copy', 'Ctrl+');

    expect(store.binding(COPY)).toBe('C');
  });

  /**
   * `rebind` refuses to make one, so this answers for what it cannot refuse: a
   * preferences file edited by hand, and a shipped default that lands on a taken key.
   */
  it('reports a collision it had no chance to refuse', () => {
    preferences.write('devnotes.shortcut.canvas.pin', 'C');

    expect(store.conflicts(EVERY)).toEqual([{ accelerator: 'C', actions: [COPY, PIN] }]);
  });

  it('reports nothing while every action has a keystroke of its own', () => {
    expect(store.conflicts(EVERY)).toEqual([]);
  });
});
