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
import { STATUS_CLASSES } from './http-status.data';
import { HTTP_STATUS_REFERENCE } from './http-status.reference';

const COLUMNS = referenceColumns('tools.http-status.columns', {
  code: '60px',
  name: '27%',
  meaning: null,
  by: '104px',
});

/** A reference: nothing typed to keep, so no result and no sample. */
@Component({
  selector: 'app-http-status-tool',
  imports: [ReferenceCellsDirective, ReferenceTableComponent, TranslocoPipe],
  templateUrl: './http-status-tool.component.html',
  styleUrl: './http-status-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class HttpStatusToolComponent implements Tool {
  protected readonly query = toolState(referenceQueryKey('http-status'), '');
  protected readonly columns = COLUMNS;
  protected readonly groups = referenceHeadings(STATUS_CLASSES, (id) => `tools.http-status.groups.${id}`);
  protected readonly rows = referenceRows(HTTP_STATUS_REFERENCE);

  clear(): void {
    this.query.set('');
  }
}
