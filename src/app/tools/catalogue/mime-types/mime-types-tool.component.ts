import { ChangeDetectionStrategy, Component, computed } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { referenceWords } from '@core/services/tools/reference-words';
import { Tool } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import {
  ReferenceCellsDirective,
  ReferenceGroup,
  ReferenceRow,
  ReferenceTableComponent,
  referenceColumns,
} from '@tools/ui/reference-table/reference-table.component';
import { MIME_GROUPS, MIME_TYPES, MimeType, MimeTypeWords, mimeGroup } from './mime-types.data';

interface MimeRow extends ReferenceRow {
  readonly mime: MimeType;
  readonly description: string;
}

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
  protected readonly query = toolState('mime-types.query', '');
  protected readonly columns = COLUMNS;
  protected readonly groups = GROUPS;

  private readonly words = referenceWords<MimeTypeWords>({
    fr: () => import('./mime-types.fr.json'),
    en: () => import('./mime-types.en.json'),
  });

  /** `.webp` and `webp` both find image/webp: each extension is searched with its dot and without. */
  protected readonly rows = computed<readonly MimeRow[]>(() => {
    const words = this.words();
    if (words === null) return [];

    return MIME_TYPES.map((mime) => {
      const description = words.types[mime.type] ?? '';
      return {
        key: mime.type,
        group: mimeGroup(mime.type),
        mime,
        description,
        searchable: [
          mime.type,
          description,
          ...mime.extensions,
          ...mime.extensions.map((extension) => extension.slice(1)),
        ],
      };
    });
  });

  clear(): void {
    this.query.set('');
  }
}
