import { ChangeDetectionStrategy, Component, computed } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { headingOf, referenceHeadings, referenceWords } from '@core/services/tools/reference-words';
import { Tool } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import {
  ReferenceCellsDirective,
  ReferenceColumn,
  ReferenceRow,
  ReferenceTableComponent,
  matchesSearchable,
} from '@tools/ui/reference-table/reference-table.component';
import { PORTS, PORT_RANGES, Port, PortWords, portRange } from './ports.data';

interface PortRow extends ReferenceRow {
  readonly port: Port;
  readonly note: string;
}

const COLUMNS: readonly ReferenceColumn[] = ['port', 'transport', 'service', 'note'].map((id) => ({
  id,
  labelKey: `tools.ports.columns.${id}`,
}));

const DIGITS = /^\d+$/;

/** A number is read as the start of a port, so that « 54 » finds 5432 and not 1521's note. */
function matchesPort(row: PortRow, query: string): boolean {
  const wanted = query.trim();
  return DIGITS.test(wanted) ? row.key.startsWith(wanted) : matchesSearchable(row, query);
}

@Component({
  selector: 'app-ports-tool',
  imports: [ReferenceCellsDirective, ReferenceTableComponent, TranslocoPipe],
  templateUrl: './ports-tool.component.html',
  styleUrl: './ports-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class PortsToolComponent implements Tool {
  protected readonly query = toolState('ports.query', '');
  protected readonly columns = COLUMNS;
  protected readonly matches = matchesPort;

  private readonly words = referenceWords<PortWords>({
    fr: () => import('./ports.fr.json'),
    en: () => import('./ports.en.json'),
  });

  protected readonly groups = referenceHeadings(PORT_RANGES, (id) => `tools.ports.ranges.${id}`);

  protected readonly rows = computed<readonly PortRow[]>(() => {
    const words = this.words();
    if (words === null) return [];
    const headings = this.groups();

    return PORTS.map((port) => {
      const note = words.ports[port.number] ?? '';
      const range = portRange(port.number);
      return {
        key: String(port.number),
        group: range,
        port,
        note,
        searchable: [String(port.number), port.service, note, headingOf(headings, range)],
      };
    });
  });

  clear(): void {
    this.query.set('');
  }
}
