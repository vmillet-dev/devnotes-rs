import { ChangeDetectionStrategy, Component, computed, inject, viewChild } from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import { ToolsRepository } from '@core/data/tools.repository';
import { InstantAnswer, InstantForms, Magnitude } from '@core/model/tool-answers.model';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { TranslationRef } from '@core/services/i18n/translation-ref.model';
import { ClockService } from '@core/services/time/clock.service';
import { liveResult } from '@core/services/tools/live-result';
import { Tool, ToolResult } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import { spanRef } from '@core/utils/relative-time.util';
import {
  Segment,
  SegmentedChoiceComponent,
} from '@shared/controls/segmented-choice/segmented-choice.component';
import { ResultRowComponent } from '@tools/ui/result-row/result-row.component';
import { DurationsComponent } from './durations/durations.component';

type Read = Extract<InstantAnswer, { kind: 'read' }>;
type Tab = 'convert' | 'durations';

const TABS: readonly Segment[] = (['convert', 'durations'] as const).map((id) => ({
  id,
  labelKey: `tools.dates.tabs.${id}`,
}));

const AUTO = 'auto';

const MAGNITUDES: readonly Segment[] = (
  [AUTO, 'seconds', 'milliseconds', 'microseconds', 'nanoseconds'] as const
).map((id) => ({ id, labelKey: `tools.dates.magnitudes.${id}` }));

/** A form of the instant: its value as Rust wrote it, or words the front writes (a weekday, "il y a 3 j"). */
interface FormRow {
  readonly id: string;
  readonly value?: string;
  readonly words?: TranslationRef;
  /** How long ago reads as of now, and is pasted nowhere. */
  readonly copyable: boolean;
}

interface FormGroup {
  readonly id: 'unix' | 'formats' | 'landmarks';
  readonly rows: readonly FormRow[];
}

function groupsOf(forms: InstantForms, now: Date): FormGroup[] {
  const value = (id: string, text: string | null): FormRow[] =>
    text === null ? [] : [{ id, value: text, copyable: true }];
  return [
    {
      id: 'unix',
      rows: [
        ...value('unixSeconds', forms.unixSeconds),
        ...value('unixMilliseconds', forms.unixMilliseconds),
        ...value('unixMicroseconds', forms.unixMicroseconds),
        ...value('unixNanoseconds', forms.unixNanoseconds),
      ],
    },
    {
      id: 'formats',
      rows: [
        ...value('isoUtc', forms.isoUtc),
        ...value('isoLocal', forms.isoLocal),
        ...value('rfc2822', forms.rfc2822),
      ],
    },
    {
      id: 'landmarks',
      rows: [
        ...(forms.epochMilliseconds === null
          ? []
          : [{ id: 'relative', words: spanRef(forms.epochMilliseconds, now), copyable: false }]),
        { id: 'weekday', words: { key: `tools.dates.weekdays.${forms.weekday}` }, copyable: true },
        ...value('weekDate', forms.weekDate),
        ...value('week', String(forms.week)),
        ...value('ordinalDate', forms.ordinalDate),
        ...value('dayOfYear', String(forms.dayOfYear)),
      ],
    },
  ];
}

/** "Lundi 8 novembre 1971, 05:25:55": the display of the instant, in the language on screen. */
function longDate(epochMilliseconds: number, lang: string): string {
  const at = new Date(epochMilliseconds);
  const day = new Intl.DateTimeFormat(lang, {
    weekday: 'long',
    day: 'numeric',
    month: 'long',
    year: 'numeric',
  }).format(at);
  const time = new Intl.DateTimeFormat(lang, {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hourCycle: 'h23',
  }).format(at);
  return `${day.charAt(0).toLocaleUpperCase(lang)}${day.slice(1)}, ${time}`;
}

/** Every reading and every form is Rust's; how long ago is the front's, so it ages without a round trip. */
@Component({
  selector: 'app-dates-tool',
  imports: [DurationsComponent, ResultRowComponent, SegmentedChoiceComponent, TranslocoPipe],
  templateUrl: './dates-tool.component.html',
  styleUrl: './dates-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class DatesToolComponent implements Tool {
  private readonly repository = inject(ToolsRepository);
  private readonly notifier = inject(ErrorNotifier);
  private readonly clock = inject(ClockService);
  private readonly transloco = inject(TranslocoService);

  private readonly lang = toSignal(this.transloco.langChanges$, {
    initialValue: this.transloco.getActiveLang(),
  });

  protected readonly tab = toolState<Tab>('dates.tab', 'convert');
  protected readonly text = toolState('dates.text', '');
  /** The other tab's fields, filled by the sample and emptied by Vider from here. */
  private readonly from = toolState('dates.from', '');
  private readonly to = toolState('dates.to', '');
  private readonly duration = toolState('dates.duration', '');

  private readonly durations = viewChild(DurationsComponent);

  protected readonly tabs = TABS;
  protected readonly magnitude = toolState<Magnitude | null>('dates.magnitude', null);

  protected readonly magnitudes = MAGNITUDES;

  protected readonly answer = liveResult(
    () => (this.text().trim() === '' ? undefined : { text: this.text(), magnitude: this.magnitude() }),
    (request) => this.repository.describeInstant(request),
  );

  protected readonly read = computed<Read | null>(() => {
    const answer = this.answer.value();
    return answer?.kind === 'read' ? answer : null;
  });

  protected readonly problem = computed(() => {
    const answer = this.answer.value();
    return answer === null || answer.kind === 'read' ? null : answer;
  });

  protected readonly reading = computed(() => {
    const read = this.read();
    return read
      ? { key: read.readAs === 'unix' ? read.magnitude : read.readAs, guessed: read.guessed }
      : null;
  });

  /** Offered for a number, and kept while one is forced so it can be set back. */
  protected readonly numeric = computed(() => this.magnitude() !== null || this.read()?.readAs === 'unix');

  protected readonly groups = computed(() => {
    const read = this.read();
    return read ? groupsOf(read.forms, this.clock.now()) : [];
  });

  /** The card's title: the local date in words, or its ISO form past what `Date` can hold. */
  protected readonly heading = computed(() => {
    const forms = this.read()?.forms;
    if (!forms) return null;
    return {
      date:
        forms.epochMilliseconds === null ? forms.isoLocal : longDate(forms.epochMilliseconds, this.lang()),
      offset: forms.localOffset,
    };
  });

  readonly result = computed<ToolResult | null>(() =>
    this.tab() === 'durations' ? (this.durations()?.result() ?? null) : this.converted(),
  );

  private readonly converted = computed<ToolResult | null>(() => {
    const forms = this.read()?.forms;
    return forms
      ? {
          title: { key: 'tools.dates.noteTitle', params: { instant: forms.isoUtc } },
          kind: 'snippet',
          language: 'txt',
          content: [
            `Unix: ${forms.unixSeconds}`,
            `Unix ms: ${forms.unixMilliseconds}`,
            `ISO 8601: ${forms.isoUtc}`,
            `ISO 8601: ${forms.isoLocal}`,
            ...(forms.rfc2822 === null ? [] : [`RFC 2822: ${forms.rfc2822}`]),
          ].join('\n'),
        }
      : null;
  });

  sample(): void {
    this.magnitude.set(null);
    this.text.set('1700000000');
    this.from.set('2026-03-12 09:00');
    this.to.set('2026-10-04 16:41');
    this.duration.set('PT1H30M');
  }

  clear(): void {
    this.text.set('');
    this.magnitude.set(null);
    this.from.set('');
    this.to.set('');
    this.duration.set('');
  }

  protected onTab(id: string): void {
    this.tab.set(id as Tab);
  }

  protected async takeNow(): Promise<void> {
    const now = await this.notifier.attempt('errors.toolFailed', () => this.repository.currentInstant());
    if (now !== null) {
      this.magnitude.set(null);
      this.text.set(now);
    }
  }

  protected onInput(event: Event): void {
    this.text.set((event.target as HTMLInputElement).value);
  }

  protected onMagnitude(id: string): void {
    this.magnitude.set(id === AUTO ? null : (id as Magnitude));
  }
}
