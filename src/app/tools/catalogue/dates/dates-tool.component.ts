import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
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
import { OutputRowComponent } from '@tools/ui/output-row/output-row.component';

type Read = Extract<InstantAnswer, { kind: 'read' }>;

const AUTO = 'auto';

const MAGNITUDES: readonly Segment[] = (
  [AUTO, 'seconds', 'milliseconds', 'microseconds', 'nanoseconds'] as const
).map((id) => ({ id, labelKey: `tools.dates.magnitudes.${id}` }));

/** A form of the instant: its value as Rust wrote it, or words the front writes (a weekday, "il y a 3 j"). */
interface FormRow {
  readonly id: string;
  readonly value?: string;
  readonly words?: TranslationRef;
  readonly meta?: TranslationRef;
}

function rowsOf(forms: InstantForms, now: Date): FormRow[] {
  const optional = (id: string, value: string | null): FormRow[] => (value === null ? [] : [{ id, value }]);
  return [
    { id: 'unixSeconds', value: forms.unixSeconds },
    { id: 'unixMilliseconds', value: forms.unixMilliseconds },
    { id: 'unixMicroseconds', value: forms.unixMicroseconds },
    ...optional('unixNanoseconds', forms.unixNanoseconds),
    { id: 'isoUtc', value: forms.isoUtc },
    {
      id: 'isoLocal',
      value: forms.isoLocal,
      meta: { key: 'tools.dates.localMeta', params: { offset: forms.localOffset } },
    },
    ...optional('rfc2822', forms.rfc2822),
    ...(forms.epochMilliseconds === null
      ? []
      : [{ id: 'relative', words: spanRef(forms.epochMilliseconds, now) }]),
    { id: 'weekday', words: { key: `tools.dates.weekdays.${forms.weekday}` } },
    {
      id: 'week',
      value: forms.weekDate,
      meta: { key: 'tools.dates.weekMeta', params: { week: forms.week } },
    },
    {
      id: 'dayOfYear',
      value: forms.ordinalDate,
      meta: { key: 'tools.dates.dayMeta', params: { day: forms.dayOfYear } },
    },
  ];
}

/** Every reading and every form is Rust's; how long ago is the front's, so it ages without a round trip. */
@Component({
  selector: 'app-dates-tool',
  imports: [OutputRowComponent, SegmentedChoiceComponent, TranslocoPipe],
  templateUrl: './dates-tool.component.html',
  styleUrl: './dates-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class DatesToolComponent implements Tool {
  private readonly repository = inject(ToolsRepository);
  private readonly notifier = inject(ErrorNotifier);
  private readonly clock = inject(ClockService);

  protected readonly text = toolState('dates.text', '');
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

  protected readonly rows = computed(() => {
    const read = this.read();
    return read ? rowsOf(read.forms, this.clock.now()) : [];
  });

  readonly result = computed<ToolResult | null>(() => {
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

  clear(): void {
    this.text.set('');
    this.magnitude.set(null);
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
