import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import { ToolsRepository } from '@core/data/tools.repository';
import { CronAnswer } from '@core/model/tool-answers.model';
import { ClockService } from '@core/services/time/clock.service';
import { liveResult } from '@core/services/tools/live-result';
import { Tool, ToolResult } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import { spanRef } from '@core/utils/relative-time.util';
import { cronSentence } from './cron-sentence';

type Read = Extract<CronAnswer, { kind: 'read' }>;

/** Rust takes the expression apart and counts its runs; the sentence is built here, from keys. */
@Component({
  selector: 'app-cron-tool',
  imports: [TranslocoPipe],
  templateUrl: './cron-tool.component.html',
  styleUrl: './cron-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CronToolComponent implements Tool {
  private readonly repository = inject(ToolsRepository);
  private readonly transloco = inject(TranslocoService);
  private readonly clock = inject(ClockService);

  protected readonly expression = toolState('cron.expression', '');
  /** `null` is the machine's zone. */
  protected readonly zone = toolState<string | null>('cron.zone', null);
  protected readonly check = toolState('cron.check', '');

  /** Where the caret stands in the expression: the field around it is lit. */
  protected readonly caret = signal<number | null>(null);
  protected readonly query = signal('');

  /** The sentence is written in the language on screen: its file loaded, it is written again. */
  private readonly translation = toSignal(this.transloco.selectTranslation());

  protected readonly answer = liveResult(
    () =>
      this.expression().trim() === ''
        ? undefined
        : { expression: this.expression(), zone: this.zone(), check: this.check() },
    (request) => this.repository.describeCron(request),
  );

  protected readonly found = liveResult(
    () => (this.query().trim() === '' ? undefined : this.query()),
    (query) => this.repository.searchTimeZones(query),
  );

  protected readonly read = computed<Read | null>(() => {
    const answer = this.answer.value();
    return answer?.kind === 'read' ? answer : null;
  });

  protected readonly reboot = computed(() => this.answer.value()?.kind === 'reboot');

  protected readonly unknownZone = computed(() => {
    const answer = this.answer.value();
    return answer?.kind === 'unknownZone' ? answer.zone : null;
  });

  protected readonly problem = computed(() => {
    this.translation();
    const answer = this.answer.value();
    if (answer?.kind !== 'refused') return null;
    return {
      problem: answer.problem,
      field: answer.field,
      at: answer.at,
      message: this.transloco.translate<string>(`tools.cron.problems.${answer.problem}`, {
        token: answer.token,
        range: answer.field
          ? this.transloco.translate<string>(`tools.cron.fields.${answer.field}.range`)
          : '',
      }),
    };
  });

  protected readonly sentence = computed(() => {
    this.translation();
    const read = this.read();
    if (!read) return '';
    const list = new Intl.ListFormat(this.transloco.getActiveLang(), { type: 'conjunction' });
    return cronSentence(
      read.fields,
      (key, params) => this.transloco.translate<string>(key, params),
      (items) => list.format(items),
    );
  });

  /** None when a macro was expanded: the offsets are the expansion's, not the text typed. */
  protected readonly litField = computed(() => {
    const [read, caret] = [this.read(), this.caret()];
    if (!read || read.expanded !== null || caret === null) return null;
    return read.fields.find((field) => caret >= field.start && caret <= field.end)?.field ?? null;
  });

  protected readonly runs = computed(() =>
    (this.read()?.runs ?? []).map((run) => ({
      ...run,
      day: `tools.cron.days.${run.weekday % 7}`,
      relative: run.epochMilliseconds === null ? null : spanRef(run.epochMilliseconds, this.clock.now()),
    })),
  );

  readonly result = computed<ToolResult | null>(() => {
    this.translation();
    const [read, asked] = [this.read(), this.answer.answered()];
    if (!read || !asked) return null;
    const runs = read.runs.length
      ? [
          '',
          this.transloco.translate<string>('tools.cron.noteRuns', { zone: read.zone }),
          ...read.runs.map((run) => `${run.date} ${run.time}`),
        ]
      : [];
    return {
      title: { key: 'tools.cron.noteTitle', params: { expression: asked.expression.trim() } },
      kind: 'snippet',
      language: 'txt',
      content: [asked.expression.trim(), `${this.sentence()}.`, ...runs].join('\n'),
    };
  });

  clear(): void {
    this.expression.set('');
    this.check.set('');
    this.query.set('');
    this.caret.set(null);
  }

  protected onInput(event: Event): void {
    this.expression.set((event.target as HTMLInputElement).value);
    this.onCaret(event);
  }

  protected onCaret(event: Event): void {
    this.caret.set((event.target as HTMLInputElement).selectionStart);
  }

  protected onCheck(event: Event): void {
    this.check.set((event.target as HTMLInputElement).value);
  }

  protected onQuery(event: Event): void {
    this.query.set((event.target as HTMLInputElement).value);
  }

  protected pickZone(zone: string | null): void {
    this.zone.set(zone);
    this.query.set('');
  }
}
