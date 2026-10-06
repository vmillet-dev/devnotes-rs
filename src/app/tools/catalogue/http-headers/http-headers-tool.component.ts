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
import { HEADER_GROUPS, HTTP_HEADERS, HttpHeader, HttpHeaderWords } from './http-headers.data';

interface HeaderRow extends ReferenceRow {
  readonly header: HttpHeader;
  readonly does: string;
}

const COLUMNS: readonly ReferenceColumn[] = ['name', 'direction', 'does', 'by'].map((id) => ({
  id,
  labelKey: `tools.http-headers.columns.${id}`,
}));

@Component({
  selector: 'app-http-headers-tool',
  imports: [ReferenceCellsDirective, ReferenceTableComponent, TranslocoPipe],
  templateUrl: './http-headers-tool.component.html',
  styleUrl: './http-headers-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class HttpHeadersToolComponent implements Tool {
  protected readonly query = toolState('http-headers.query', '');
  protected readonly columns = COLUMNS;

  private readonly words = referenceWords<HttpHeaderWords>({
    fr: () => import('./http-headers.fr.json'),
    en: () => import('./http-headers.en.json'),
  });

  protected readonly groups = referenceHeadings(HEADER_GROUPS, (id) => `tools.http-headers.groups.${id}`);

  protected readonly rows = computed<readonly HeaderRow[]>(() => {
    const words = this.words();
    if (words === null) return [];
    const headings = this.groups();

    return HTTP_HEADERS.map((header) => {
      const does = words.headers[header.name.toLowerCase()] ?? '';
      return {
        key: header.name,
        group: header.group,
        header,
        does,
        searchable: [header.name, does, header.example, header.by, headingOf(headings, header.group)],
      };
    });
  });

  clear(): void {
    this.query.set('');
  }
}
