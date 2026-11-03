import { describe, expect, it } from 'vitest';
import { DialogLayer, dialogRung } from './dialog.model';

/** The order of the rungs is the file's whole content, so that is what is asserted. */
const IN_ORDER: readonly DialogLayer[] = [
  'editor',
  'titlebar',
  'app',
  'settings',
  'update',
  'palette',
  'fields',
  'zoom',
  'passphrase',
];

describe('dialogRung', () => {
  it('gives every layer a rung of its own', () => {
    const rungs = IN_ORDER.map(dialogRung);

    expect(new Set(rungs).size).toBe(IN_ORDER.length);
  });

  /** A rung is both the `z-index` and the Escape priority: one list, two behaviours. */
  it('rises with the order the layers are declared in', () => {
    const rungs = IN_ORDER.map(dialogRung);

    expect(rungs).toEqual([...rungs].sort((left, right) => left - right));
  });

  /**
   * The banners of `layout/` sit at 80 and must stay above every modal — they are
   * triggered from inside one. That is what the base well below it is for.
   */
  it('stays under the banners, which are raised from inside a dialog', () => {
    expect(Math.max(...IN_ORDER.map(dialogRung))).toBeLessThan(80);
  });

  it('draws a dialog opened from another one in front of it', () => {
    // The fields form is created after the palette and drawn over it.
    expect(dialogRung('fields')).toBeGreaterThan(dialogRung('palette'));
    // The passphrase prompt blocks an operation already under way.
    expect(dialogRung('passphrase')).toBeGreaterThan(dialogRung('settings'));
    // A zoomed image is opened from the editor.
    expect(dialogRung('zoom')).toBeGreaterThan(dialogRung('editor'));
  });

  /** The titlebar stays in reach over a full-screen note, so what it opens lands in front. */
  it('draws the titlebar menus and the help panels over the editor', () => {
    expect(dialogRung('titlebar')).toBeGreaterThan(dialogRung('editor'));
    expect(dialogRung('app')).toBeGreaterThan(dialogRung('titlebar'));
  });
});
