import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { PreferencesService } from '@core/services/preferences/preferences.service';
import { GLOBAL_ACTIONS, Rebindable } from '@core/services/shortcuts/shortcut.model';
import { SettingsDraftStore } from './settings-draft.store';
import { SettingsStore } from './settings.store';

const PALETTE = GLOBAL_ACTIONS[0];
const COPY: Rebindable = { id: 'canvas.copy', labelKey: 'shortcuts.canvas.copy', fallback: 'C' };
const PIN: Rebindable = { id: 'canvas.pin', labelKey: 'shortcuts.canvas.pin', fallback: 'P' };
const EVERY = [...GLOBAL_ACTIONS, COPY, PIN];

describe('SettingsDraftStore', () => {
  let draft: SettingsDraftStore;
  let settings: SettingsStore;
  let preferences: PreferencesService;

  const theme = (): string | undefined => document.documentElement.dataset['theme'];

  beforeEach(() => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({});
    draft = TestBed.inject(SettingsDraftStore);
    settings = TestBed.inject(SettingsStore);
    preferences = TestBed.inject(PreferencesService);
  });

  it('answers what is stored until something is staged over it', () => {
    expect(draft.value('density')).toBe('comfortable');

    draft.set('density', 'compact');

    expect(draft.value('density')).toBe('compact');
    expect(settings.density()).toBe('comfortable');
  });

  /** ⚠️ Or the panel would claim there is something to apply after a switch was toggled twice. */
  it('goes clean again when a value is put back to what is stored', () => {
    draft.set('closeToTray', false);
    expect(draft.isDirty()).toBe(true);

    draft.set('closeToTray', true);

    expect(draft.isDirty()).toBe(false);
    expect(draft.pendingCount()).toBe(0);
  });

  it('writes every staged setting on apply, and nothing before', () => {
    draft.set('density', 'compact');
    draft.set('copyConfirmation', false);
    expect(draft.pendingCount()).toBe(2);

    draft.apply();

    expect(settings.density()).toBe('compact');
    expect(settings.copyConfirmation()).toBe(false);
    expect(draft.isDirty()).toBe(false);
  });

  it('drops everything on cancel, leaving the settings where they were', () => {
    draft.set('density', 'compact');

    draft.cancel();

    expect(settings.density()).toBe('comfortable');
    expect(draft.value('density')).toBe('comfortable');
  });

  /**
   * ⚠️ A preview is not a write. Nobody picks a theme without seeing it, so the two that
   * are worth seeing show on `<html>` straight away — and cancelling puts the previous
   * appearance back with nothing to roll back.
   */
  describe('the appearance it previews', () => {
    beforeEach(() => {
      TestBed.flushEffects();
    });

    it('shows a staged theme on the document without storing it', () => {
      draft.set('theme', 'light');
      TestBed.flushEffects();

      expect(theme()).toBe('light');
      expect(preferences.read('devnotes.theme')).toBeNull();
    });

    it('puts the previous appearance back on cancel', () => {
      // Stored rather than defaulted: "system" resolves to light where `matchMedia` is
      // absent, which is the same answer the preview gives and proves nothing.
      settings.setTheme('dark');
      draft.set('theme', 'light');
      TestBed.flushEffects();
      expect(theme()).toBe('light');

      draft.cancel();
      TestBed.flushEffects();

      expect(theme()).toBe('dark');
    });

    it('keeps it on apply, now from what was written', () => {
      draft.set('theme', 'light');

      draft.apply();
      TestBed.flushEffects();

      expect(theme()).toBe('light');
      expect(settings.theme()).toBe('light');
    });

    /** ⚠️ A half-captured global shortcut live across the machine is why nothing else previews. */
    it('previews nothing but the theme and the density', () => {
      draft.set('paletteShortcut', 'Ctrl+Shift+K');
      TestBed.flushEffects();

      expect(settings.paletteShortcut()).toBe('Ctrl+Alt+P');
    });
  });

  describe('the keys it stages', () => {
    it('answers the staged keystroke while the stores still answer the old one', () => {
      draft.rebind(COPY, 'Y', EVERY);

      expect(draft.binding(COPY)).toBe('Y');
      expect(preferences.read('devnotes.shortcut.canvas.copy')).toBeNull();
    });

    it('writes both paths on apply', () => {
      draft.rebind(COPY, 'Y', EVERY);
      draft.rebind(PALETTE, 'Ctrl+Shift+K', EVERY);

      draft.apply();

      expect(preferences.read('devnotes.shortcut.canvas.copy')).toBe('Y');
      expect(settings.paletteShortcut()).toBe('Ctrl+Shift+K');
    });

    /** ⚠️ The same rule the store applies, read through what is staged rather than stored. */
    it('refuses a keystroke another action was staged onto a moment ago', () => {
      draft.rebind(PIN, 'Y', EVERY);

      expect(draft.rebind(COPY, 'Y', EVERY)).toEqual({ kind: 'taken', by: PIN });
    });

    it('sees a conflict between something staged and something stored', () => {
      draft.rebind(PIN, 'Y', EVERY);
      draft.apply();
      draft.rebind(COPY, 'Y', EVERY);

      expect(draft.rebind(COPY, 'Y', EVERY)).toEqual({ kind: 'taken', by: PIN });
    });

    it('stages a reset without writing the default over the file', () => {
      draft.rebind(COPY, 'Y', EVERY);
      draft.apply();

      draft.resetBinding(COPY);
      expect(draft.binding(COPY)).toBe('C');
      expect(preferences.read('devnotes.shortcut.canvas.copy')).toBe('Y');

      draft.apply();

      expect(preferences.read('devnotes.shortcut.canvas.copy')).toBeNull();
    });

    it('goes clean again when a key is put back where it already was', () => {
      draft.rebind(COPY, 'Y', EVERY);
      draft.resetBinding(COPY);

      expect(draft.isDirty()).toBe(false);
    });
  });
});
