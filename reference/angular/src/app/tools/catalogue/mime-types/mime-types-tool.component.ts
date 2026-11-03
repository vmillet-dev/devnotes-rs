import { ChangeDetectionStrategy, Component } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { referenceRows } from '@core/services/tools/reference-words';
import { referenceQueryKey } from '@core/services/tools/reference.model';
import { Tool } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import {
  ReferenceCellsDirective,
  ReferenceGroup,
  ReferenceTableComponent,
  referenceColumns,
} from '@tools/ui/reference-table/reference-table.component';
import { MIME_GROUPS } from './mime-types.data';
import { MIME_TYPES_REFERENCE } from './mime-types.reference';

const COLUMNS = referenceColumns('tools.mime-types.columns', {
  type: '36%',
  extensions: '24%',
  description: null,
});

/** The top-level type is its own heading, in no language. */
const GROUPS: readonly ReferenceGroup[] = MIME_GROUPS.map((id) => ({ id, label: `${id}/*` }));

@Component({
  selector: 'app-mime-types-tool',
  imports: [ReferenceCellsDirective, ReferenceTableComponent, TranslocoPipe],
  templateUrl: './mime-types-tool.component.html',
  styleUrl: './mime-types-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class MimeTypesToolComponent implements Tool {
  protected readonly query = toolState(referenceQueryKey('mime-types'), '');
  protected readonly columns = COLUMNS;
  protected readonly groups = GROUPS;
  protected readonly rows = referenceRows(MIME_TYPES_REFERENCE);

  clear(): void {
    this.query.set('');
  }
}
