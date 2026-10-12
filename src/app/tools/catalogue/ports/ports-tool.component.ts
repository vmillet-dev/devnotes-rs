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
import { PORT_RANGES } from './ports.data';
import { PORTS_REFERENCE, matchesPort } from './ports.reference';

const COLUMNS = referenceColumns('tools.ports.columns', {
  port: '68px',
  transport: '100px',
  service: '30%',
  note: null,
});

@Component({
  selector: 'app-ports-tool',
  imports: [ReferenceCellsDirective, ReferenceTableComponent, TranslocoPipe],
  templateUrl: './ports-tool.component.html',
  styleUrl: './ports-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class PortsToolComponent implements Tool {
  protected readonly query = toolState(referenceQueryKey('ports'), '');
  protected readonly columns = COLUMNS;
  protected readonly matches = matchesPort;
  protected readonly groups = referenceHeadings(PORT_RANGES, (id) => `tools.ports.ranges.${id}`);
  protected readonly rows = referenceRows(PORTS_REFERENCE);

  clear(): void {
    this.query.set('');
  }
}
