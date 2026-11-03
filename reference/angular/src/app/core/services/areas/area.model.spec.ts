import { describe, expect, it } from 'vitest';
import {
  AREAS,
  AREA_ACTIONS,
  AREA_SHORTCUT_GROUP,
  RAIL_ACTION,
  areaForKey,
  isSearchKey,
  isToolSearchKey,
} from './area.model';

describe('areaForKey', () => {
  const key = (code: string, init: KeyboardEventInit = {}) =>
    areaForKey(new KeyboardEvent('keydown', { code, key: '?', ctrlKey: true, ...init }));

  it('reads Ctrl and the rank of an area, from the row or the pad', () => {
    expect(key('Digit1')).toBe('notes');
    expect(key('Digit2')).toBe('tools');
    expect(key('Numpad2')).toBe('tools');
  });

  /** On AZERTY the unshifted digit row types `&é"`, and the position is what the sheet names. */
  it('goes by the key position, not by the character it types', () => {
    expect(areaForKey(new KeyboardEvent('keydown', { code: 'Digit1', key: '&', ctrlKey: true }))).toBe(
      'notes',
    );
  });

  it('answers nothing past the last area, nor for another chord', () => {
    expect(key(`Digit${AREAS.length + 1}`)).toBeNull();
    expect(key('Digit1', { ctrlKey: false })).toBeNull();
    expect(key('Digit1', { altKey: true })).toBeNull();
    expect(key('Digit1', { shiftKey: true })).toBeNull();
    expect(key('KeyA')).toBeNull();
  });

  it('documents one key per area, in the switch order, then the searches and the rail', () => {
    expect(AREA_SHORTCUT_GROUP.shortcuts.map((entry) => entry.keys)).toEqual([
      ...AREAS.map((_, index) => ['Ctrl', String(index + 1)]),
      ['Ctrl', 'K'],
      ['Ctrl', 'Shift', 'T'],
      ['Ctrl', 'B'],
    ]);
  });

  /** The rail's key can be moved, and the sheet then draws the key it is bound to. */
  it('lists the rail once, as an action of every area that can be rebound', () => {
    expect(AREA_ACTIONS).toEqual([RAIL_ACTION]);
    expect(AREA_SHORTCUT_GROUP.shortcuts.at(-1)?.action).toBe(RAIL_ACTION);
  });

  it('knows the tools search by its letter, whatever the layout puts under it', () => {
    const chord = (init: KeyboardEventInit) => isToolSearchKey(new KeyboardEvent('keydown', init));

    expect(chord({ key: 'T', ctrlKey: true, shiftKey: true })).toBe(true);
    expect(chord({ key: 't', ctrlKey: true })).toBe(false);
    expect(chord({ key: 'T', ctrlKey: true, shiftKey: true, altKey: true })).toBe(false);
  });
});

describe('isSearchKey', () => {
  it('reads Ctrl+K, and nothing that carries Shift or Alt as well', () => {
    const chord = (init: KeyboardEventInit) => isSearchKey(new KeyboardEvent('keydown', init));

    expect(chord({ key: 'k', ctrlKey: true })).toBe(true);
    expect(chord({ key: 'K', metaKey: true })).toBe(true);
    expect(chord({ key: 'k', ctrlKey: true, shiftKey: true })).toBe(false);
    expect(chord({ key: 'k' })).toBe(false);
  });
});
