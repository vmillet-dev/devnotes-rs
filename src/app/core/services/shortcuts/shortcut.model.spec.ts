import { describe, expect, it } from 'vitest';
import {
  DEFAULT_SHORTCUTS,
  acceleratorFromEvent,
  acceleratorKeys,
  canvasKeystrokeFromEvent,
  isAccelerator,
} from './shortcut.model';

function keydown(init: KeyboardEventInit): KeyboardEvent {
  return new KeyboardEvent('keydown', init);
}

describe('acceleratorFromEvent', () => {
  it('spells a combination the way the native side reads it', () => {
    expect(acceleratorFromEvent(keydown({ code: 'KeyP', ctrlKey: true, altKey: true }))).toBe('Ctrl+Alt+P');
  });

  it('keeps the modifiers in a stable order, whatever the keyboard reports', () => {
    const event = keydown({ code: 'KeyK', shiftKey: true, ctrlKey: true, metaKey: true });

    expect(acceleratorFromEvent(event)).toBe('Ctrl+Shift+Super+K');
  });

  it('reads a digit and a function key by name', () => {
    expect(acceleratorFromEvent(keydown({ code: 'Digit7', ctrlKey: true }))).toBe('Ctrl+7');
    expect(acceleratorFromEvent(keydown({ code: 'F5', altKey: true }))).toBe('Alt+F5');
  });

  it('says nothing while only a modifier is held', () => {
    expect(acceleratorFromEvent(keydown({ code: 'ControlLeft', ctrlKey: true }))).toBeNull();
  });

  it('refuses a key pressed without any modifier', () => {
    expect(acceleratorFromEvent(keydown({ code: 'KeyP' }))).toBeNull();
  });

  it('refuses a key the native side could not name', () => {
    expect(acceleratorFromEvent(keydown({ code: 'Lang1', ctrlKey: true }))).toBeNull();
  });

  it('reads the key by position, not by the character it produces', () => {
    expect(acceleratorFromEvent(keydown({ code: 'KeyA', key: 'q', ctrlKey: true }))).toBe('Ctrl+A');
  });
});

describe('canvasKeystrokeFromEvent', () => {
  it('reads the printed key, modifiers first', () => {
    expect(canvasKeystrokeFromEvent(keydown({ key: 'c', code: 'KeyC' }))).toBe('C');
    expect(canvasKeystrokeFromEvent(keydown({ key: 'b', code: 'KeyB', ctrlKey: true }))).toBe('Ctrl+B');
  });

  it('reads no keystroke in the keydown Chromium fires when a suggestion is picked', () => {
    expect(canvasKeystrokeFromEvent(new Event('keydown'))).toBeNull();
  });
});

describe('isAccelerator', () => {
  it('accepts what the defaults are made of', () => {
    expect(isAccelerator(DEFAULT_SHORTCUTS.palette)).toBe(true);
  });

  it('rejects a lone key, and a modifier nobody knows', () => {
    expect(isAccelerator('P')).toBe(false);
    expect(isAccelerator('Hyper+P')).toBe(false);
  });

  it('rejects a trailing token that is not a key', () => {
    expect(isAccelerator('Ctrl+Alt+')).toBe(false);
  });
});

describe('acceleratorKeys', () => {
  it('splits a combination into one key per cap', () => {
    expect(acceleratorKeys('Ctrl+Alt+P')).toEqual(['Ctrl', 'Alt', 'P']);
  });

  it('drops what a stray separator leaves behind', () => {
    expect(acceleratorKeys('Ctrl++P')).toEqual(['Ctrl', 'P']);
  });
});
