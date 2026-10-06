import { ChangeDetectionStrategy, Component, computed } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { headingOf, referenceHeadings, referenceWords } from '@core/services/tools/reference-words';
import { Tool } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import {
  ReferenceCellsDirective,
  ReferenceRow,
  ReferenceTableComponent,
  referenceColumns,
  matchesSearchable,
} from '@tools/ui/reference-table/reference-table.component';
import { SIGNALS, SIGNAL_GROUPS, SignalAction, SignalWords, UnixSignal } from './signals.data';

interface SignalRow extends ReferenceRow {
  readonly signal: UnixSignal;
  readonly use: string;
  /** macOS's number, Linux's when the signal does not differ; `null` where macOS has none. */
  readonly macos: string | null;
}

const ACTIONS: readonly SignalAction[] = ['terminate', 'core', 'ignore', 'stop', 'continue'];

const COLUMNS = referenceColumns('tools.signals.columns', {
  number: '64px',
  name: '104px',
  action: '136px',
  catchable: '128px',
  use: null,
  macos: '68px',
});

const DIGITS = /^\d+$/;

/** `34–64`, the real-time signals, holds every number between. */
function holds(numbers: string | null, wanted: number): boolean {
  if (numbers === null) return false;
  const [from = Number.NaN, to = from] = numbers.split('–').map(Number);
  return wanted >= from && wanted <= to;
}

/** A number is a signal's number exactly: « 9 » is SIGKILL, not 19 nor 29. */
function matchesSignal(row: SignalRow, query: string): boolean {
  const wanted = query.trim();
  return DIGITS.test(wanted) ? holds(row.signal.linux, Number(wanted)) : matchesSearchable(row, query);
}

@Component({
  selector: 'app-signals-tool',
  imports: [ReferenceCellsDirective, ReferenceTableComponent, TranslocoPipe],
  templateUrl: './signals-tool.component.html',
  styleUrl: './signals-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SignalsToolComponent implements Tool {
  protected readonly query = toolState('signals.query', '');
  protected readonly columns = COLUMNS;
  protected readonly matches = matchesSignal;

  private readonly words = referenceWords<SignalWords>({
    fr: () => import('./signals.fr.json'),
    en: () => import('./signals.en.json'),
  });

  protected readonly groups = referenceHeadings(SIGNAL_GROUPS, (id) => `tools.signals.groups.${id}`);
  /** Searched too: « core » finds every signal that leaves a core dump. */
  private readonly actionLabels = referenceHeadings(ACTIONS, (id) => `tools.signals.actions.${id}`);

  protected readonly rows = computed<readonly SignalRow[]>(() => {
    const words = this.words();
    if (words === null) return [];
    const [headings, actions] = [this.groups(), this.actionLabels()];

    return SIGNALS.map((signal) => {
      const use = words.signals[signal.name] ?? '';
      return {
        key: signal.name,
        group: signal.group,
        signal,
        use,
        macos: signal.macos === undefined ? signal.linux : signal.macos,
        searchable: [signal.name, use, headingOf(headings, signal.group), headingOf(actions, signal.action)],
      };
    });
  });

  clear(): void {
    this.query.set('');
  }
}
