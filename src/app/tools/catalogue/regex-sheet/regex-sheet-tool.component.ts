import { ChangeDetectionStrategy, Component, computed } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { headingOf, referenceHeadings, referenceWords } from '@core/services/tools/reference-words';
import { Tool } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import { IconComponent } from '@shared/icon/icon.component';
import {
  ReferenceCellsDirective,
  ReferenceRow,
  ReferenceTableComponent,
  referenceColumns,
  matchesSearchable,
} from '@tools/ui/reference-table/reference-table.component';
import { REGEX_ENTRIES, REGEX_GROUPS, RegexEntry, RegexWords } from './regex-sheet.data';

export interface RegexRow extends ReferenceRow {
  readonly entry: RegexEntry;
  readonly what: string;
  readonly jsNote: string | null;
  readonly pcreNote: string | null;
}

const COLUMNS = referenceColumns('tools.regex-sheet.columns', {
  syntax: '18%',
  what: null,
  example: '31%',
  js: '112px',
  pcre: '112px',
});

const METACHARACTERS = /[\\^$.*+?()[\]{}|]/;

/**
 * A query written like a pattern is looked for in the syntax, case and all: `\b` is not `\B`.
 * Words — « lookbehind », « paresseux » — are searched as everywhere else.
 */
export function matchesRegex(row: RegexRow, query: string): boolean {
  const wanted = query.trim();
  return METACHARACTERS.test(wanted) ? row.key.includes(wanted) : matchesSearchable(row, query);
}

@Component({
  selector: 'app-regex-sheet-tool',
  imports: [IconComponent, ReferenceCellsDirective, ReferenceTableComponent, TranslocoPipe],
  templateUrl: './regex-sheet-tool.component.html',
  styleUrl: './regex-sheet-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class RegexSheetToolComponent implements Tool {
  protected readonly query = toolState('regex-sheet.query', '');
  protected readonly columns = COLUMNS;
  protected readonly matches = matchesRegex;

  private readonly words = referenceWords<RegexWords>({
    fr: () => import('./regex-sheet.fr.json'),
    en: () => import('./regex-sheet.en.json'),
  });

  protected readonly groups = referenceHeadings(REGEX_GROUPS, (id) => `tools.regex-sheet.groups.${id}`);

  protected readonly rows = computed<readonly RegexRow[]>(() => {
    const words = this.words();
    if (words === null) return [];
    const headings = this.groups();

    return REGEX_ENTRIES.map((entry) => {
      const said = words.entries[entry.syntax];
      const what = said?.what ?? '';
      return {
        key: entry.syntax,
        group: entry.group,
        entry,
        what,
        jsNote: said?.js ?? null,
        pcreNote: said?.pcre ?? null,
        searchable: [entry.syntax, what, said?.js ?? '', said?.pcre ?? '', headingOf(headings, entry.group)],
      };
    });
  });

  clear(): void {
    this.query.set('');
  }
}
