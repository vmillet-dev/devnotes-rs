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
import { SIGNAL_GROUPS } from './signals.data';
import { SIGNALS_REFERENCE, matchesSignal } from './signals.reference';

const COLUMNS = referenceColumns('tools.signals.columns', {
  number: '64px',
  name: '104px',
  action: '136px',
  catchable: '128px',
  use: null,
  macos: '68px',
});

@Component({
  selector: 'app-signals-tool',
  imports: [ReferenceCellsDirective, ReferenceTableComponent, TranslocoPipe],
  templateUrl: './signals-tool.component.html',
  styleUrl: './signals-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SignalsToolComponent implements Tool {
  protected readonly query = toolState(referenceQueryKey('signals'), '');
  protected readonly columns = COLUMNS;
  protected readonly matches = matchesSignal;
  protected readonly groups = referenceHeadings(SIGNAL_GROUPS, (id) => `tools.signals.groups.${id}`);
  protected readonly rows = referenceRows(SIGNALS_REFERENCE);

  clear(): void {
    this.query.set('');
  }
}
