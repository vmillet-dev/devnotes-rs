import { ChangeDetectionStrategy, Component, computed } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { headingOf, referenceHeadings, referenceWords } from '@core/services/tools/reference-words';
import { Tool } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import {
  ReferenceCellsDirective,
  ReferenceRow,
  ReferenceTableComponent,
  referenceColumns,
  matchesSearchable,
} from '@tools/ui/reference-table/reference-table.component';
import {
  ASCII_CODES,
  ASCII_GROUPS,
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

const COLUMNS = referenceColumns('tools.ascii.columns', {
  decimal: '56px',
  hex: '56px',
  octal: '56px',
  binary: '96px',
  character: '60px',
  name: null,
  escape: '84px',
  ctrl: '60px',
});

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

@Component({
  selector: 'app-ascii-tool',
  imports: [ReferenceCellsDirective, ReferenceTableComponent, TranslocoPipe],
  templateUrl: './ascii-tool.component.html',
  styleUrl: './ascii-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class AsciiToolComponent implements Tool {
  protected readonly query = toolState('ascii.query', '');
  protected readonly columns = COLUMNS;
  protected readonly matches = matchesAscii;

  private readonly words = referenceWords<AsciiWords>({
    fr: () => import('./ascii.fr.json'),
    en: () => import('./ascii.en.json'),
  });

  protected readonly groups = referenceHeadings(ASCII_GROUPS, (id) => `tools.ascii.groups.${id}`);

  /** Computed from the code points: the words alone are a file. */
  protected readonly rows = computed<readonly AsciiRow[]>(() => {
    const words = this.words();
    if (words === null) return [];
    const headings = this.groups();

    return ASCII_CODES.map((code) => {
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
          headingOf(headings, group),
        ],
      };
    });
  });

  clear(): void {
    this.query.set('');
  }
}
