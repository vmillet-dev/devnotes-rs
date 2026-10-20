import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  Injector,
  afterNextRender,
  computed,
  effect,
  inject,
  input,
  output,
  signal,
  viewChild,
} from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import {
  CONSOLE_FILTERS,
  ConsoleEntry,
  ConsoleFilter,
  consoleCounts,
  consoleKindOf,
  consoleTime,
  originKey,
  secretSegments,
} from '@core/model/http-console.model';
import { stopMessageOf, stopTitleOf } from '@core/model/http.model';
import { errorKeyOf } from '@core/services/errors/error-notifier.service';
import { HttpConsoleStore } from '@core/services/http/http-console.store';
import { TranslationRef } from '@core/services/i18n/translation-ref.model';
import { IconComponent } from '@shared/icon/icon.component';

/** A line as drawn: its text is a sentence to translate, or words as they were said. */
interface ConsoleRow {
  readonly time: string;
  readonly filter: Exclude<ConsoleFilter, 'all'>;
  /** `→`, `←`, `log`, `test`…: what kind of line, in a column of its own. */
  readonly mark: string;
  readonly tone: string;
  readonly said: TranslationRef | null;
  /** After it, set apart: what stopped a script, when it is not its own words. */
  readonly then: TranslationRef | null;
  readonly words: readonly { readonly text: string; readonly secret: boolean }[];
  /** Under a request: its headers, masked. */
  readonly sub: string | null;
  readonly origin: TranslationRef | null;
  /** The send the response shows, its own traffic: its exchange opens on the timeline. */
  readonly exchange: boolean;
}

function rowOf(entry: ConsoleEntry, requestName: string, current: string | null): ConsoleRow {
  const { line } = entry;
  const base = {
    time: consoleTime(entry.at),
    filter: consoleKindOf(line),
    said: null,
    then: null,
    words: [],
    sub: null,
    exchange: false,
    origin:
      'by' in line && line.by
        ? { key: originKey(line, line.by), params: { name: line.by.name } }
        : { key: 'http.console.origins.request', params: { name: requestName } },
  } satisfies Partial<ConsoleRow>;
  const own = 'by' in line && !line.by && entry.sendId === current;
  switch (line.kind) {
    case 'out':
      return {
        ...base,
        mark: '→',
        tone: 'out',
        words: secretSegments(`${line.method} ${line.url}`),
        sub: line.headers.map((header) => `${header.key}: ${header.value}`).join('   ') || null,
        exchange: own,
      };
    case 'in':
      return {
        ...base,
        mark: '←',
        tone: line.status >= 400 ? 'in bad' : 'in',
        said: { key: 'http.console.in', params: { ...line } },
        exchange: own,
      };
    case 'failed':
      return {
        ...base,
        mark: '✗',
        tone: 'bad',
        said: { key: errorKeyOf(line.code, line.params) ?? 'http.send.failed', params: line.params },
      };
    case 'log':
      return { ...base, mark: line.level, tone: `log ${line.level}`, words: secretSegments(line.text) };
    case 'test':
      return {
        ...base,
        mark: 'test',
        tone: 'bad',
        said:
          line.expected !== null || line.actual !== null
            ? {
                key: 'http.console.test',
                params: { ...line, expected: line.expected ?? '—', actual: line.actual ?? '—' },
              }
            : { key: 'http.console.testMessage', params: { name: line.name, message: line.message ?? '' } },
      };
    case 'stopped':
      return {
        ...base,
        mark: '✗',
        tone: 'bad',
        said: stopTitleOf(line.stop),
        then: stopMessageOf(line.stop),
        words: stopMessageOf(line.stop) === null ? secretSegments(` : ${line.stop.message ?? ''}`) : [],
        origin: null,
      };
    case 'missed':
      return {
        ...base,
        mark: '!',
        tone: 'bad',
        said: { key: 'http.console.missed', params: { path: line.path, variable: line.variable } },
      };
  }
}

/**
 * The console under a request's answer: its traffic — a script's sends included —, the scripts'
 * logs, the tests that failed and what stopped, each with its time and whose it is. Folded, a
 * line that counts them.
 */
@Component({
  selector: 'app-http-console',
  imports: [IconComponent, TranslocoPipe],
  templateUrl: './http-console.component.html',
  styleUrl: './http-console.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class HttpConsoleComponent {
  readonly tabKey = input.required<string>();
  readonly requestName = input.required<string>();
  /** The send the response shows. */
  readonly currentSendId = input<string | null>(null);
  /** « Ouvrir l'échange › » on a line of that send. */
  readonly exchangeOpened = output<void>();

  protected readonly store = inject(HttpConsoleStore);
  private readonly injector = inject(Injector);
  private readonly list = viewChild<ElementRef<HTMLElement>>('list');

  protected readonly filters = CONSOLE_FILTERS;
  protected readonly filter = signal<ConsoleFilter>('all');
  private readonly entries = computed(() => this.store.linesOf(this.tabKey()));
  protected readonly counts = computed(() => consoleCounts(this.entries()));
  protected readonly rows = computed(() => {
    const filter = this.filter();
    const current = this.currentSendId();
    return this.entries()
      .map((entry) => rowOf(entry, this.requestName(), current))
      .filter((row) => filter === 'all' || row.filter === filter);
  });

  constructor() {
    // The last line in sight as lines come: a console is read from its bottom.
    effect(() => {
      if (this.rows().length === 0 || !this.store.open()) return;
      afterNextRender(
        {
          write: () => {
            const list = this.list()?.nativeElement;
            if (list) list.scrollTop = list.scrollHeight;
          },
        },
        { injector: this.injector },
      );
    });
  }

  protected clear(): void {
    this.store.clear(this.tabKey());
  }
}
