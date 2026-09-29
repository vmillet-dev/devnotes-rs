import { describe, expect, it } from 'vitest';
import { AREAS, AREA_SHORTCUT_GROUP, areaForKey } from './area.model';

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

  it('documents one key per area, in the switch order', () => {
    expect(AREA_SHORTCUT_GROUP.shortcuts.map((entry) => entry.keys)).toEqual(
      AREAS.map((_, index) => ['Ctrl', String(index + 1)]),
    );
  });
});
