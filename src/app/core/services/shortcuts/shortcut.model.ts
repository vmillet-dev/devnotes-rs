import { DEFAULT_SHORTCUTS } from '@core/ipc/bindings';
import type { ShortcutBindings } from '@core/ipc/bindings';

export type { ShortcutBindings };

/**
 * The three accelerators the native side registers before the front end exists — read
 * from it rather than retyped. ⚠️ It still has defaults of its own, and must: without
 * them `Ctrl+Alt+P` is dead for the length of the first render, which is exactly the
 * second it gets used from another application.
 */
export { DEFAULT_SHORTCUTS };

/**
 * An action a key can be moved to: what it is stored under, what to call it, and the
 * accelerator it ships with.
 *
 * ⚠️ It carries its own fallback, so `ShortcutBindingsStore` holds no list of its own —
 * the canvas ones are declared in the table that binds them and the global ones just
 * below, and a store that also kept a copy would be a third place to add a key to.
 */
export interface Rebindable {
  readonly id: string;
  readonly labelKey: string;
  readonly fallback: string;
}

/**
 * ⚠️ The three registered with the operating system, first come first served across the
 * machine. They are stored in `AppSettings` because the native command takes them as a
 * block, where a canvas key is one preference of its own.
 */
export const GLOBAL_ACTIONS: readonly Rebindable[] = [
  { id: 'palette', labelKey: 'shortcuts.global.palette', fallback: DEFAULT_SHORTCUTS.palette },
  { id: 'capture', labelKey: 'shortcuts.global.capture', fallback: DEFAULT_SHORTCUTS.capture },
  { id: 'newNote', labelKey: 'shortcuts.global.newNote', fallback: DEFAULT_SHORTCUTS.newNote },
];

/** `keys` is split because each one is rendered as its own `<kbd>`. */
export interface ShortcutEntry {
  readonly keys: readonly string[];
  readonly labelKey: string;
  /**
   * Present when the row can be moved, which is what lets the read-only sheet draw the
   * key that is bound rather than the one that shipped. `keys` is then its default.
   */
  readonly action?: Rebindable | undefined;
}

export interface ShortcutGroup {
  readonly id: string;
  readonly labelKey: string;
  readonly shortcuts: readonly ShortcutEntry[];
}

/** A modifier alone is not a finished combination. */
const MODIFIER_CODES = new Set([
  'ControlLeft',
  'ControlRight',
  'AltLeft',
  'AltRight',
  'AltGraph',
  'ShiftLeft',
  'ShiftRight',
  'MetaLeft',
  'MetaRight',
]);

/** Letters, digits and function keys are handled separately, by pattern. */
const NAMED_KEYS = new Set([
  'Space',
  'Tab',
  'Enter',
  'Backspace',
  'Delete',
  'Insert',
  'Home',
  'End',
  'PageUp',
  'PageDown',
  'ArrowUp',
  'ArrowDown',
  'ArrowLeft',
  'ArrowRight',
  'Backquote',
  'Minus',
  'Equal',
  'BracketLeft',
  'BracketRight',
  'Backslash',
  'Semicolon',
  'Quote',
  'Comma',
  'Period',
  'Slash',
  'CapsLock',
  'PrintScreen',
  'ScrollLock',
  'Pause',
]);

const MODIFIER_NAMES = new Set(['Ctrl', 'Alt', 'Shift', 'Super']);

/**
 * ⚠️ `KeyboardEvent.code` is the key's position, not the character it produces: a
 * shortcut set on AZERTY stays in the same place on QWERTY, which `event.key` would
 * not guarantee.
 */
function keyName(code: string): string | null {
  const named = /^Key([A-Z])$/.exec(code)?.[1] ?? /^Digit(\d)$/.exec(code)?.[1];

  return named ?? (isKeyName(code) ? code : null);
}

/** `P` and not `KeyP`, both being accepted by the native parser. */
function isKeyName(name: string): boolean {
  return (
    /^[A-Z]$/.test(name) || /^\d$/.test(name) || /^F([1-9]|1\d|2[0-4])$/.test(name) || NAMED_KEYS.has(name)
  );
}

/**
 * ⚠️ At least one modifier is required: a **global** shortcut without one would swallow
 * that key in every application on the machine, typing included.
 */
export function acceleratorFromEvent(event: KeyboardEvent): string | null {
  if (MODIFIER_CODES.has(event.code)) return null;

  const modifiers: string[] = [];
  if (event.ctrlKey) modifiers.push('Ctrl');
  if (event.altKey) modifiers.push('Alt');
  if (event.shiftKey) modifiers.push('Shift');
  if (event.metaKey) modifiers.push('Super');
  if (modifiers.length === 0) return null;

  const key = keyName(event.code);

  return key ? [...modifiers, key].join('+') : null;
}

/** `event.key` names them; the codes above name the same keys by position. */
const MODIFIER_KEYS = new Set(['Control', 'Alt', 'AltGraph', 'Shift', 'Meta']);

/**
 * A canvas keystroke, modifiers first — `C`, `Ctrl+B`, `Delete`.
 *
 * ⚠️ The **printed** key, where a global shortcut reads the key's position. The opposite
 * trade, and a deliberate one: a global combination must survive a layout change because
 * the native side parses it back by position, while `C` on the canvas is read off the
 * keycap by someone looking at this window — on AZERTY, position `KeyA` is the key
 * labelled `Q`, and aligning the board from the key marked `A` is what a reader expects.
 *
 * ⚠️ `Ctrl` covers ⌘ too: the table is written with one modifier name, and `Super` means
 * something of its own only where a shortcut leaves the window.
 */
export function canvasKeystrokeFromEvent(event: KeyboardEvent): string | null {
  if (MODIFIER_KEYS.has(event.key)) return null;

  const modifiers: string[] = [];
  if (event.ctrlKey || event.metaKey) modifiers.push('Ctrl');
  if (event.altKey) modifiers.push('Alt');
  if (event.shiftKey) modifiers.push('Shift');

  // ⚠️ Upper-cased, so a caps-locked keyboard answers the same key as a bare one.
  const key = event.key.length === 1 ? event.key.toUpperCase() : event.key;

  return isCanvasKey(key) ? [...modifiers, key].join('+') : null;
}

export function isAccelerator(value: string): boolean {
  const tokens = value.split('+').map((token) => token.trim());
  if (tokens.length < 2) return false;

  const key = tokens.at(-1);
  const modifiers = tokens.slice(0, -1);

  return key !== undefined && modifiers.every((modifier) => MODIFIER_NAMES.has(modifier)) && isKeyName(key);
}

/**
 * ⚠️ The keyboard would stop answering if these moved: the arrows are the grid's own
 * navigation, Tab is how the window is crossed, and Escape is the way out of every other
 * thing on screen. `+` and the space are refused for a duller reason — one is the
 * separator and the other is written as a blank nobody could read back in a field.
 */
const RESERVED_KEYS = new Set(['Tab', 'Escape', 'ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight', '+', ' ']);

/** One printed character, or a key `event.key` names in full. */
function isCanvasKey(key: string): boolean {
  if (RESERVED_KEYS.has(key)) return false;

  return key.length === 1 || NAMED_KEYS.has(key) || /^F([1-9]|1\d|2[0-4])$/.test(key);
}

/**
 * ⚠️ A canvas key may be **bare**, where a global one may not: it answers only while the
 * canvas has the keyboard, so `C` on the copy costs nothing outside the window.
 */
export function isCanvasAccelerator(value: string): boolean {
  const tokens = value.split('+').map((token) => token.trim());
  const key = tokens.at(-1);
  if (key === undefined || !isCanvasKey(key)) return false;

  return tokens.slice(0, -1).every((modifier) => MODIFIER_NAMES.has(modifier));
}

/** One key per `<kbd>`: a single string would put the `+` inside the key caps. */
export function acceleratorKeys(accelerator: string): readonly string[] {
  return accelerator
    .split('+')
    .map((token) => token.trim())
    .filter((token) => token !== '');
}
