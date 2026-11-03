export const ASCII_GROUPS = ['control', 'punctuation', 'digits', 'upper', 'lower'] as const;
export type AsciiGroup = (typeof ASCII_GROUPS)[number];

export interface AsciiControl {
  readonly abbreviation: string;
  /** The standard's own name, in English whatever the language on screen. */
  readonly name: string;
  /** The usual escape in C and its heirs, where there is one. */
  readonly escape?: string;
  /** What else it is searched by: « newline », « XOFF ». */
  readonly aliases?: readonly string[];
}

export interface AsciiWords {
  /** Keyed by the decimal code: a control's role, a symbol's name. Letters and digits have none. */
  readonly codes: Readonly<Record<string, string>>;
}

export const DELETE = 127;

export const CONTROLS: Readonly<Record<number, AsciiControl>> = {
  0: { abbreviation: 'NUL', name: 'Null', escape: '\\0' },
  1: { abbreviation: 'SOH', name: 'Start of Heading' },
  2: { abbreviation: 'STX', name: 'Start of Text' },
  3: { abbreviation: 'ETX', name: 'End of Text' },
  4: { abbreviation: 'EOT', name: 'End of Transmission', aliases: ['EOF'] },
  5: { abbreviation: 'ENQ', name: 'Enquiry' },
  6: { abbreviation: 'ACK', name: 'Acknowledge' },
  7: { abbreviation: 'BEL', name: 'Bell', escape: '\\a' },
  8: { abbreviation: 'BS', name: 'Backspace', escape: '\\b' },
  9: { abbreviation: 'HT', name: 'Horizontal Tab', escape: '\\t' },
  10: { abbreviation: 'LF', name: 'Line Feed', escape: '\\n', aliases: ['newline', 'NL'] },
  11: { abbreviation: 'VT', name: 'Vertical Tab', escape: '\\v' },
  12: { abbreviation: 'FF', name: 'Form Feed', escape: '\\f' },
  13: { abbreviation: 'CR', name: 'Carriage Return', escape: '\\r' },
  14: { abbreviation: 'SO', name: 'Shift Out' },
  15: { abbreviation: 'SI', name: 'Shift In' },
  16: { abbreviation: 'DLE', name: 'Data Link Escape' },
  17: { abbreviation: 'DC1', name: 'Device Control 1', aliases: ['XON'] },
  18: { abbreviation: 'DC2', name: 'Device Control 2' },
  19: { abbreviation: 'DC3', name: 'Device Control 3', aliases: ['XOFF'] },
  20: { abbreviation: 'DC4', name: 'Device Control 4' },
  21: { abbreviation: 'NAK', name: 'Negative Acknowledge' },
  22: { abbreviation: 'SYN', name: 'Synchronous Idle' },
  23: { abbreviation: 'ETB', name: 'End of Transmission Block' },
  24: { abbreviation: 'CAN', name: 'Cancel' },
  25: { abbreviation: 'EM', name: 'End of Medium' },
  26: { abbreviation: 'SUB', name: 'Substitute' },
  27: { abbreviation: 'ESC', name: 'Escape', escape: '\\e' },
  28: { abbreviation: 'FS', name: 'File Separator' },
  29: { abbreviation: 'GS', name: 'Group Separator' },
  30: { abbreviation: 'RS', name: 'Record Separator' },
  31: { abbreviation: 'US', name: 'Unit Separator' },
  [DELETE]: { abbreviation: 'DEL', name: 'Delete' },
};

/** The 128 code points, in order. */
export const ASCII_CODES: readonly number[] = Array.from({ length: 128 }, (_, code) => code);

export function asciiGroup(code: number): AsciiGroup {
  if (code < 32 || code === DELETE) return 'control';
  if (code >= 48 && code <= 57) return 'digits';
  if (code >= 65 && code <= 90) return 'upper';
  if (code >= 97 && code <= 122) return 'lower';
  return 'punctuation';
}

/** `^A` for 1: a control is typed with Ctrl and the character 64 above it; DEL is `^?`. */
export function ctrlKey(code: number): string | null {
  if (code === DELETE) return '^?';
  return code < 32 ? `^${String.fromCharCode(code + 64)}` : null;
}

export const hex = (code: number): string => code.toString(16).toUpperCase().padStart(2, '0');
export const octal = (code: number): string => code.toString(8).padStart(3, '0');
export const binary = (code: number): string => code.toString(2).padStart(8, '0');
