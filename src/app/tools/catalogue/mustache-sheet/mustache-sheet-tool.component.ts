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
import { MUSTACHE_GROUPS } from './mustache-sheet.data';
import { MUSTACHE_SHEET_REFERENCE } from './mustache-sheet.reference';

const COLUMNS = referenceColumns('tools.mustache-sheet.columns', {
  syntax: '25%',
  what: '32%',
  example: null,
});

/**
 * ⚠️ Every `{{…}}` here is a constant or a word of the chunk, never a main translation string:
 * Transloco would replace an unknown `{{name}}` with nothing.
 */
@Component({
  selector: 'app-mustache-sheet-tool',
  imports: [ReferenceCellsDirective, ReferenceTableComponent, TranslocoPipe],
  templateUrl: './mustache-sheet-tool.component.html',
  styleUrl: './mustache-sheet-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class MustacheSheetToolComponent implements Tool {
  protected readonly query = toolState(referenceQueryKey('mustache-sheet'), '');
  protected readonly columns = COLUMNS;
  protected readonly groups = referenceHeadings(MUSTACHE_GROUPS, (id) => `tools.mustache-sheet.groups.${id}`);
  protected readonly rows = referenceRows(MUSTACHE_SHEET_REFERENCE);

  clear(): void {
    this.query.set('');
  }
}
