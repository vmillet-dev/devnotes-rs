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
} from '@tools/ui/reference-table/reference-table.component';
import { MUSTACHE_ENTRIES, MUSTACHE_GROUPS, MustacheEntry, MustacheWords } from './mustache-sheet.data';

interface MustacheRow extends ReferenceRow {
  readonly entry: MustacheEntry;
  readonly what: string;
}

const COLUMNS = referenceColumns('tools.mustache-sheet.columns', {
  syntax: '25%',
  what: '32%',
  example: null,
});

/**
 * ⚠️ Every `{{…}}` here is a constant or a word of the chunk, never a main translation string:
 * Transloco would replace an unknown `{{name}}` with nothing.
 */
@Component({
  selector: 'app-mustache-sheet-tool',
  imports: [ReferenceCellsDirective, ReferenceTableComponent, TranslocoPipe],
  templateUrl: './mustache-sheet-tool.component.html',
  styleUrl: './mustache-sheet-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class MustacheSheetToolComponent implements Tool {
  protected readonly query = toolState('mustache-sheet.query', '');
  protected readonly columns = COLUMNS;

  private readonly words = referenceWords<MustacheWords>({
    fr: () => import('./mustache-sheet.fr.json'),
    en: () => import('./mustache-sheet.en.json'),
  });

  protected readonly groups = referenceHeadings(MUSTACHE_GROUPS, (id) => `tools.mustache-sheet.groups.${id}`);

  protected readonly rows = computed<readonly MustacheRow[]>(() => {
    const words = this.words();
    if (words === null) return [];
    const headings = this.groups();

    return MUSTACHE_ENTRIES.map((entry) => {
      const what = words.entries[entry.syntax] ?? '';
      return {
        key: entry.syntax,
        group: entry.group,
        entry,
        what,
        searchable: [entry.syntax, what, headingOf(headings, entry.group)],
      };
    });
  });

  clear(): void {
    this.query.set('');
  }
}
