import { ReferenceRow, ReferenceSource, matchesSearchable } from '@core/services/tools/reference.model';
import {
  ASCII_CODES,
  AsciiControl,
  AsciiWords,
  CONTROLS,
  asciiGroup,
  binary,
  ctrlKey,
  hex,
  octal,
} from './ascii.data';

export interface AsciiRow extends ReferenceRow {
  readonly code: number;
  readonly character: string;
  readonly control: AsciiControl | null;
  readonly hex: string;
  readonly octal: string;
  readonly binary: string;
  readonly ctrl: string | null;
  readonly words: string;
}

const SPACE = 32;

const READINGS: readonly [RegExp, number][] = [
  [/^\d{1,3}$/, 10],
  [/^(?:0x|\\x)([0-9a-f]{1,2})$/i, 16],
  [/^(?=[0-9a-f]*[a-f])([0-9a-f]{2})$/i, 16],
  [/^0o([0-7]{1,3})$/i, 8],
  [/^0b([01]{1,8})$/i, 2],
];

/** The code a query names in some base — `65`, `0x41`, `1b`, `0o101`, `0b1000001` — if it names one. */
export function codeIn(query: string): number | null {
  for (const [pattern, radix] of READINGS) {
    const match = pattern.exec(query);
    if (match) return Number.parseInt(match[1] ?? match[0], radix);
  }
  return null;
}

/**
 * One character is that character, case and all, or a one-digit code: « A » is 65 alone, « 6 »
 * both the digit and ACK. Anything longer is a code in some base, or words.
 */
export function matchesAscii(row: AsciiRow, query: string): boolean {
  if ([...query].length === 1) return row.character === query || row.key === query;
  return codeIn(query.trim()) === row.code || matchesSearchable(row, query);
}

/** What stands for the character where it cannot be drawn: `NUL`, « espace ». */
function shown(row: AsciiRow): string {
  return row.control?.abbreviation ?? (row.code === SPACE ? row.words : row.character);
}

export const ASCII_REFERENCE: ReferenceSource<AsciiWords, AsciiRow> = {
  words: {
    fr: () => import('./ascii.fr.json'),
    en: () => import('./ascii.en.json'),
  },
  /** Computed from the code points: the words alone are a file. */
  rows: (words, label) =>
    ASCII_CODES.map((code) => {
      const control = CONTROLS[code] ?? null;
      const character = String.fromCharCode(code);
      const said = words.codes[code] ?? '';
      const group = asciiGroup(code);
      const ctrl = ctrlKey(code);
      return {
        key: String(code),
        group,
        code,
        character,
        control,
        hex: hex(code),
        octal: octal(code),
        binary: binary(code),
        ctrl,
        words: said,
        copy: { value: character, what: control?.abbreviation ?? (code === SPACE ? said : character) },
        searchable: [
          ...(control
            ? [control.abbreviation, control.name, control.escape ?? '', ...(control.aliases ?? [])]
            : []),
          said,
          ctrl ?? '',
          label(`tools.ascii.groups.${group}`),
        ],
      };
    }),
  title: (row) => `${shown(row)} (${row.key})`,
  /** A letter or a digit has no words of its own: its code in hexadecimal says what it is. */
  line: (row) => [row.words || row.control?.name, `0x${row.hex}`].filter(Boolean).join(' · '),
  matches: matchesAscii,
};
