import { ChangeDetectionStrategy, Component } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { referenceHeadings, referenceRows } from '@core/services/tools/reference-words';
import { referenceQueryKey } from '@core/services/tools/reference.model';
import { Tool } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import { IconComponent } from '@shared/icon/icon.component';
import {
  ReferenceCellsDirective,
  ReferenceTableComponent,
  referenceColumns,
} from '@tools/ui/reference-table/reference-table.component';
import { MARKDOWN_GROUPS } from './markdown-sheet.data';
import { MARKDOWN_SHEET_REFERENCE } from './markdown-sheet.reference';

const COLUMNS = referenceColumns('tools.markdown-sheet.columns', {
  syntax: '15%',
  what: null,
  example: '30%',
  commonmark: '116px',
  github: '88px',
});

@Component({
  selector: 'app-markdown-sheet-tool',
  imports: [IconComponent, ReferenceCellsDirective, ReferenceTableComponent, TranslocoPipe],
  templateUrl: './markdown-sheet-tool.component.html',
  styleUrl: './markdown-sheet-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class MarkdownSheetToolComponent implements Tool {
  protected readonly query = toolState(referenceQueryKey('markdown-sheet'), '');
  protected readonly columns = COLUMNS;
  protected readonly groups = referenceHeadings(MARKDOWN_GROUPS, (id) => `tools.markdown-sheet.groups.${id}`);
  protected readonly rows = referenceRows(MARKDOWN_SHEET_REFERENCE);

  clear(): void {
    this.query.set('');
  }
}
