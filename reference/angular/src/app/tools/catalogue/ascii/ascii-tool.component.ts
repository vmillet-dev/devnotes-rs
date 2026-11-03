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
import { ASCII_GROUPS } from './ascii.data';
import { ASCII_REFERENCE, matchesAscii } from './ascii.reference';

const COLUMNS = referenceColumns('tools.ascii.columns', {
  decimal: '56px',
  hex: '56px',
  octal: '56px',
  binary: '96px',
  character: '60px',
  name: null,
  escape: '84px',
  ctrl: '60px',
});

@Component({
  selector: 'app-ascii-tool',
  imports: [ReferenceCellsDirective, ReferenceTableComponent, TranslocoPipe],
  templateUrl: './ascii-tool.component.html',
  styleUrl: './ascii-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class AsciiToolComponent implements Tool {
  protected readonly query = toolState(referenceQueryKey('ascii'), '');
  protected readonly columns = COLUMNS;
  protected readonly matches = matchesAscii;
  protected readonly groups = referenceHeadings(ASCII_GROUPS, (id) => `tools.ascii.groups.${id}`);
  protected readonly rows = referenceRows(ASCII_REFERENCE);

  clear(): void {
    this.query.set('');
  }
}
