import { ChangeDetectionStrategy, Component, computed } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { headingOf, referenceHeadings, referenceWords } from '@core/services/tools/reference-words';
import { Tool } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import { IconComponent } from '@shared/icon/icon.component';
import {
  ReferenceCellsDirective,
  ReferenceColumn,
  ReferenceRow,
  ReferenceTableComponent,
} from '@tools/ui/reference-table/reference-table.component';
import { MARKDOWN_ENTRIES, MARKDOWN_GROUPS, MarkdownEntry, MarkdownWords } from './markdown-sheet.data';

interface MarkdownRow extends ReferenceRow {
  readonly entry: MarkdownEntry;
  readonly what: string;
}

const COLUMNS: readonly ReferenceColumn[] = ['syntax', 'what', 'example', 'commonmark', 'github'].map(
  (id) => ({
    id,
    labelKey: `tools.markdown-sheet.columns.${id}`,
  }),
);

/** A space that matters is drawn `␠`, and copied as a space. */
const SPACE = '␠';

@Component({
  selector: 'app-markdown-sheet-tool',
  imports: [IconComponent, ReferenceCellsDirective, ReferenceTableComponent, TranslocoPipe],
  templateUrl: './markdown-sheet-tool.component.html',
  styleUrl: './markdown-sheet-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class MarkdownSheetToolComponent implements Tool {
  protected readonly query = toolState('markdown-sheet.query', '');
  protected readonly columns = COLUMNS;

  private readonly words = referenceWords<MarkdownWords>({
    fr: () => import('./markdown-sheet.fr.json'),
    en: () => import('./markdown-sheet.en.json'),
  });

  protected readonly groups = referenceHeadings(MARKDOWN_GROUPS, (id) => `tools.markdown-sheet.groups.${id}`);

  protected readonly rows = computed<readonly MarkdownRow[]>(() => {
    const words = this.words();
    if (words === null) return [];
    const headings = this.groups();

    return MARKDOWN_ENTRIES.map((entry) => {
      const what = words.entries[entry.syntax] ?? '';
      return {
        key: entry.syntax,
        group: entry.group,
        entry,
        what,
        ...(entry.syntax.includes(SPACE) && {
          copy: { value: entry.syntax.replaceAll(SPACE, ' '), what: entry.syntax },
        }),
        searchable: [entry.syntax, what, headingOf(headings, entry.group)],
      };
    });
  });

  clear(): void {
    this.query.set('');
  }
}
