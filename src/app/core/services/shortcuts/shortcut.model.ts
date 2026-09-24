import { DEFAULT_SHORTCUTS } from '@core/ipc/bindings';
import type { ShortcutBindings } from '@core/ipc/bindings';

export type { ShortcutBindings };

/**
 * The three accelerators the native side registers before the front end exists, read rather
 * than retyped. Rust keeps defaults of its own, or `Ctrl+Alt+P` would be dead during the first
 * render.
 */
export { DEFAULT_SHORTCUTS };

/**
 * An action a key can be moved to: its storage name, its label, and its default. The default
 * travels with it, so `ShortcutBindingsStore` keeps no third list of keys.
 */
export interface Rebindable {
  readonly id: string;
  readonly labelKey: string;
  readonly fallback: string;
}

/**
 * The three registered with the operating system, first come first served across the machine:
 * stored in `AppSettings`, since the native command takes them as a block.
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
  /** Present when the row can be moved, so the read-only sheet draws the key actually bound. */
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
 * ⚠️ `KeyboardEvent.code` is the key's position, not its character: a shortcut set on AZERTY
 * stays put on QWERTY.
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

/** At least one modifier: a global shortcut without one would swallow that key everywhere. */
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
 * A canvas keystroke, modifiers first — `C`, `Ctrl+B`, `Delete`. The printed key, where a
 * global shortcut reads the position: someone looking at this window reads `C` off the keycap,
 * and on AZERTY position `KeyA` is the key labelled `Q`. `Ctrl` covers ⌘ too.
 */
export function canvasKeystrokeFromEvent(event: KeyboardEvent): string | null {
  if (MODIFIER_KEYS.has(event.key)) return null;

  const modifiers: string[] = [];
  if (event.ctrlKey || event.metaKey) modifiers.push('Ctrl');
  if (event.altKey) modifiers.push('Alt');
  if (event.shiftKey) modifiers.push('Shift');

  // Upper-cased, so a caps-locked keyboard answers the same key.
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
 * The arrows move through the grid, Tab crosses the window and Escape leaves everything else.
 * `+` is the separator, and a space would read as a blank.
 */
const RESERVED_KEYS = new Set(['Tab', 'Escape', 'ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight', '+', ' ']);

/** One printed character, or a key `event.key` names in full. */
function isCanvasKey(key: string): boolean {
  if (RESERVED_KEYS.has(key)) return false;

  return key.length === 1 || NAMED_KEYS.has(key) || /^F([1-9]|1\d|2[0-4])$/.test(key);
}

/** A canvas key may be bare: it answers only while the canvas has the keyboard. */
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
