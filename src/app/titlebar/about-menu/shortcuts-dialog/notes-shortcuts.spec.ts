import { describe, expect, it } from 'vitest';
import { CANVAS_SHORTCUT_GROUP } from '@notes/canvas-keyboard.directive';
import fr from '@core/services/i18n/translations/fr.json';
import en from '@core/services/i18n/translations/en.json';
import { NOTES_SHORTCUT_GROUPS } from './notes-shortcuts';

function translation(file: Record<string, unknown>, key: string): unknown {
  return key.split('.').reduce<unknown>((node, part) => {
    return typeof node === 'object' && node !== null ? (node as Record<string, unknown>)[part] : undefined;
  }, file);
}

describe('NOTES_SHORTCUT_GROUPS', () => {
  const groups = NOTES_SHORTCUT_GROUPS;
  const shortcuts = groups.flatMap((group) => group.shortcuts);

  /**
   * ⚠️ The canvas group is not written here: it is derived from the table that *binds*
   * the keys, so a key cannot be documented in the sheet without being bound. The two
   * other groups are documentation only, handled by the editor and the palette.
   */
  it('takes its canvas group from the table that binds the keys', () => {
    expect(groups[0]).toBe(CANVAS_SHORTCUT_GROUP);
    expect(groups.map((group) => group.id)).toEqual(['notes.canvas', 'notes.editor', 'notes.palette']);
  });

  it('gives every group and every shortcut something to say', () => {
    for (const group of groups) {
      expect(group.shortcuts.length).toBeGreaterThan(0);
      for (const shortcut of group.shortcuts) {
        expect(shortcut.keys.length).toBeGreaterThan(0);
        expect(shortcut.keys.every((key) => key.trim().length > 0)).toBe(true);
      }
    }
  });

  /** A sheet that names a key nobody translated shows an empty row. */
  it('names only keys both locales can label', () => {
    const keys = [
      ...groups.map((group) => group.labelKey),
      ...shortcuts.map((shortcut) => shortcut.labelKey),
    ];

    for (const key of keys) {
      expect(translation(fr, key), `${key} manque en français`).toBeTypeOf('string');
      expect(translation(en, key), `${key} is missing in English`).toBeTypeOf('string');
    }
  });

  it('gives each group an id of its own', () => {
    expect(new Set(groups.map((group) => group.id)).size).toBe(groups.length);
  });
});
