import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import { ToolsRepository } from '@core/data/tools.repository';
import { CronAnswer, Run } from '@core/model/tool-answers.model';
import { ClockService } from '@core/services/time/clock.service';
import { liveResult } from '@core/services/tools/live-result';
import { Tool, ToolResult } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import { spanRef } from '@core/utils/relative-time.util';
import { cronSentence } from './cron-sentence';

type Read = Extract<CronAnswer, { kind: 'read' }>;

/** The windows one asks about, filled into the two fields from what « now » is in the zone. */
const SHORTCUTS = ['today', 'tomorrow', 'weekend', 'fridayAfternoon'] as const;
type Shortcut = (typeof SHORTCUTS)[number];

/** A calendar date some days on: plain arithmetic on a date Rust gave, no zone in it. */
function daysOn(date: string, days: number): string {
  const [year = 0, month = 1, day = 1] = date.split('-').map(Number);
  return new Date(Date.UTC(year, month - 1, day + days)).toISOString().slice(0, 10);
}

/**
 * The two ends of a shortcut, as they are typed. `weekday` is 1 for Monday: on a Saturday or
 * a Sunday « ce week-end » is the one under way, and « vendredi » the next.
 */
export function shortcutWindow(shortcut: Shortcut, now: Pick<Run, 'date' | 'weekday'>): [string, string] {
  const at = (days: number, time: string) => `${daysOn(now.date, days)} ${time}`;
  switch (shortcut) {
    case 'today':
      return [at(0, '00:00'), at(1, '00:00')];
    case 'tomorrow':
      return [at(1, '00:00'), at(2, '00:00')];
    case 'weekend': {
      const saturday = 6 - now.weekday;
      return [at(saturday, '00:00'), at(saturday + 2, '00:00')];
    }
    case 'fridayAfternoon': {
      const friday = now.weekday <= 5 ? 5 - now.weekday : 12 - now.weekday;
      return [at(friday, '12:00'), at(friday, '18:00')];
    }
  }
}

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
  /** The two ends of the window the runs are listed in; empty is now, and the next ten. */
  protected readonly from = toolState('cron.from', '');
  protected readonly until = toolState('cron.until', '');
  protected readonly shortcuts = SHORTCUTS;

  /** Where the caret stands in the expression: the field around it is lit. */
  protected readonly caret = signal<number | null>(null);
  protected readonly query = signal('');

  /** The sentence is written in the language on screen: its file loaded, it is written again. */
  private readonly translation = toSignal(this.transloco.selectTranslation());

  protected readonly answer = liveResult(
    () =>
      this.expression().trim() === ''
        ? undefined
        : {
            expression: this.expression(),
            zone: this.zone(),
            check: this.check(),
            from: this.from(),
            until: this.until(),
          },
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
    if (read?.expanded !== null || caret === null) return null;
    return read.fields.find((field) => caret >= field.start && caret <= field.end)?.field ?? null;
  });

  /** The window as Rust counted it, or what stops it from being counted. */
  protected readonly counted = computed(() => {
    const window = this.read()?.window ?? null;
    return window?.kind === 'counted' ? window : null;
  });

  protected readonly windowProblem = computed(() => {
    const window = this.read()?.window ?? null;
    return window !== null && window.kind !== 'counted' ? window : null;
  });

  /** A window lists its own runs: all of them, or its first and its last around a count. */
  protected readonly runs = computed(() => this.shown(this.counted()?.first ?? this.read()?.runs ?? []));
  protected readonly lastRuns = computed(() => this.shown(this.counted()?.last ?? []));

  /** The runs a long window holds between the ones it lists. */
  protected readonly between = computed(() => {
    const window = this.counted();
    return window && window.last.length > 0 ? window.total - window.first.length - window.last.length : 0;
  });

  /** The two ends in words, for the sentence that counts: « vendredi 2026-10-16 12:00 ». */
  protected readonly ends = computed(() => {
    this.translation();
    const window = this.counted();
    if (!window) return null;
    const said = (end: Run) =>
      `${this.transloco.translate<string>(`tools.cron.days.${end.weekday % 7}`)} ${end.date} ${end.time.slice(0, 5)}`;
    return { from: said(window.from), until: said(window.until) };
  });

  /** The nearest runs around a moment that is none, as they are read: date and time. */
  protected readonly nearest = computed(() => {
    const verdict = this.read()?.check ?? null;
    if (verdict?.kind !== 'checked' || verdict.matches) return null;
    const said = (run: Run | null) => (run === null ? null : `${run.date} ${run.time}`);
    return { before: said(verdict.before), after: said(verdict.after) };
  });

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

  sample(): void {
    this.expression.set('*/15 9-18 * * 1-5');
    this.caret.set(null);
  }

  clear(): void {
    this.expression.set('');
    this.check.set('');
    this.from.set('');
    this.until.set('');
    this.query.set('');
    this.caret.set(null);
  }

  protected onEnd(end: 'from' | 'until', event: Event): void {
    (end === 'from' ? this.from : this.until).set((event.target as HTMLInputElement).value);
  }

  /** A shortcut fills the two fields, as typing would: they are read in the zone chosen. */
  protected pick(shortcut: Shortcut, now: Run): void {
    const [from, until] = shortcutWindow(shortcut, now);
    this.from.set(from);
    this.until.set(until);
  }

  private shown(runs: readonly Run[]) {
    return runs.map((run) => ({
      ...run,
      day: `tools.cron.days.${run.weekday % 7}`,
      relative: run.epochMilliseconds === null ? null : spanRef(run.epochMilliseconds, this.clock.now()),
    }));
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
