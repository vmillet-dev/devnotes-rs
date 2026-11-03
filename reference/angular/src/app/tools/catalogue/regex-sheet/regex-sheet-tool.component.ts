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
import { REGEX_GROUPS } from './regex-sheet.data';
import { REGEX_SHEET_REFERENCE, matchesRegex } from './regex-sheet.reference';

const COLUMNS = referenceColumns('tools.regex-sheet.columns', {
  syntax: '18%',
  what: null,
  example: '31%',
  js: '112px',
  pcre: '112px',
});

@Component({
  selector: 'app-regex-sheet-tool',
  imports: [IconComponent, ReferenceCellsDirective, ReferenceTableComponent, TranslocoPipe],
  templateUrl: './regex-sheet-tool.component.html',
  styleUrl: './regex-sheet-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class RegexSheetToolComponent implements Tool {
  protected readonly query = toolState(referenceQueryKey('regex-sheet'), '');
  protected readonly columns = COLUMNS;
  protected readonly matches = matchesRegex;
  protected readonly groups = referenceHeadings(REGEX_GROUPS, (id) => `tools.regex-sheet.groups.${id}`);
  protected readonly rows = referenceRows(REGEX_SHEET_REFERENCE);

  clear(): void {
    this.query.set('');
  }
}
