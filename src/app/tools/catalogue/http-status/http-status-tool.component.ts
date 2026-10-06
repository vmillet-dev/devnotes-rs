import { ChangeDetectionStrategy, Component, computed } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { headingOf, referenceHeadings, referenceWords } from '@core/services/tools/reference-words';
import { Tool } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import {
  ReferenceCellsDirective,
  ReferenceColumn,
  ReferenceRow,
  ReferenceTableComponent,
} from '@tools/ui/reference-table/reference-table.component';
import { HTTP_STATUSES, HttpStatus, HttpStatusWords, STATUS_CLASSES, statusClass } from './http-status.data';

interface StatusRow extends ReferenceRow {
  readonly status: HttpStatus;
  readonly meaning: string;
  readonly use: string | null;
}

const COLUMNS: readonly ReferenceColumn[] = ['code', 'name', 'meaning', 'by'].map((id) => ({
  id,
  labelKey: `tools.http-status.columns.${id}`,
}));

/** A reference: nothing typed to keep, so no result and no sample. */
@Component({
  selector: 'app-http-status-tool',
  imports: [ReferenceCellsDirective, ReferenceTableComponent, TranslocoPipe],
  templateUrl: './http-status-tool.component.html',
  styleUrl: './http-status-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class HttpStatusToolComponent implements Tool {
  protected readonly query = toolState('http-status.query', '');
  protected readonly columns = COLUMNS;

  private readonly words = referenceWords<HttpStatusWords>({
    fr: () => import('./http-status.fr.json'),
    en: () => import('./http-status.en.json'),
  });

  protected readonly groups = referenceHeadings(STATUS_CLASSES, (id) => `tools.http-status.groups.${id}`);

  protected readonly rows = computed<readonly StatusRow[]>(() => {
    const words = this.words();
    if (words === null) return [];
    const headings = this.groups();

    return HTTP_STATUSES.map((status) => {
      const group = statusClass(status.code);
      const said = words.codes[status.code];
      const meaning = said?.meaning ?? '';
      const use = said?.use ?? null;
      return {
        key: String(status.code),
        group,
        status,
        meaning,
        use,
        searchable: [
          String(status.code),
          status.name,
          meaning,
          use ?? '',
          status.by,
          headingOf(headings, group),
        ],
      };
    });
  });

  clear(): void {
    this.query.set('');
  }
}
