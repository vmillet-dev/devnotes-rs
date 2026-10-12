import { ChangeDetectionStrategy, Component } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { referenceHeadings, referenceRows } from '@core/services/tools/reference-words';
import { referenceQueryKey } from '@core/services/tools/reference.model';
import { Tool } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import {
  ReferenceCellsDirective,
  ReferenceTableComponent,
  referenceColumns,
} from '@tools/ui/reference-table/reference-table.component';
import { HEADER_GROUPS } from './http-headers.data';
import { HTTP_HEADERS_REFERENCE } from './http-headers.reference';

const COLUMNS = referenceColumns('tools.http-headers.columns', {
  name: '31%',
  direction: '96px',
  does: null,
  by: '136px',
});

@Component({
  selector: 'app-http-headers-tool',
  imports: [ReferenceCellsDirective, ReferenceTableComponent, TranslocoPipe],
  templateUrl: './http-headers-tool.component.html',
  styleUrl: './http-headers-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class HttpHeadersToolComponent implements Tool {
  protected readonly query = toolState(referenceQueryKey('http-headers'), '');
  protected readonly columns = COLUMNS;
  protected readonly groups = referenceHeadings(HEADER_GROUPS, (id) => `tools.http-headers.groups.${id}`);
  protected readonly rows = referenceRows(HTTP_HEADERS_REFERENCE);

  clear(): void {
    this.query.set('');
  }
}
