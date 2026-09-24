import { describe, expect, it } from 'vitest';
import { DialogLayer, dialogRung } from './dialog.model';

/** The order of the rungs is the file's whole content, so that is what is asserted. */
const IN_ORDER: readonly DialogLayer[] = [
  'app',
  'editor',
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

  /**
   * A help panel covers the page, so the only way to a note while one is up is a
   * global shortcut — which comes from outside the application. The note has to arrive
   * in front of the help, not behind it.
   */
  it('draws a note opened while a help panel is up in front of it', () => {
    expect(dialogRung('editor')).toBeGreaterThan(dialogRung('app'));
  });
});
