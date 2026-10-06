import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import { referenceWords } from '@core/services/tools/reference-words';
import { Tool } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import {
  ReferenceCellsDirective,
  ReferenceColumn,
  ReferenceGroup,
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
  private readonly transloco = inject(TranslocoService);

  protected readonly query = toolState('http-status.query', '');
  protected readonly columns = COLUMNS;

  private readonly words = referenceWords<HttpStatusWords>({
    fr: () => import('./http-status.fr.json'),
    en: () => import('./http-status.en.json'),
  });

  /** Read so that the headings follow the language, and the file once it has landed. */
  private readonly translation = toSignal(this.transloco.selectTranslation());

  /** Translated here rather than in the template: « redirection » finds every 3xx by its heading. */
  protected readonly groups = computed<readonly ReferenceGroup[]>(() => {
    this.translation();
    return STATUS_CLASSES.map((id) => ({
      id,
      label: this.transloco.translate(`tools.http-status.groups.${id}`),
    }));
  });

  protected readonly rows = computed<readonly StatusRow[]>(() => {
    const words = this.words();
    if (words === null) return [];
    const headings = new Map(this.groups().map((group) => [group.id, group.label]));

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
          headings.get(group) ?? '',
        ],
      };
    });
  });

  clear(): void {
    this.query.set('');
  }
}
