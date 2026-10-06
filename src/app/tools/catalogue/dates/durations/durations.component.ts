import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import { ToolsRepository } from '@core/data/tools.repository';
import { CalendarGap, DurationParts, GapAnswer, GapField } from '@core/model/tool-answers.model';
import { ClockService } from '@core/services/time/clock.service';
import { liveResult } from '@core/services/tools/live-result';
import { ToolResult } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import { ResultRowComponent } from '@tools/ui/result-row/result-row.component';

type Measured = Extract<GapAnswer, { kind: 'measured' }>;

const DATE_UNITS = ['years', 'months', 'days'] as const;
const CLOCK_UNITS = ['hours', 'minutes', 'seconds'] as const;
const PART_UNITS = ['years', 'months', 'weeks', 'days', 'hours', 'minutes', 'seconds'] as const;

/** "2026-10-04 16:41": the local wall clock, as the fields are typed. */
function wallClock(at: Date): string {
  const two = (value: number) => String(value).padStart(2, '0');
  return `${at.getFullYear()}-${two(at.getMonth() + 1)}-${two(at.getDate())} ${two(at.getHours())}:${two(at.getMinutes())}`;
}

/**
 * The gap between two dates and an ISO 8601 duration, both Rust's. What reads them aloud — « 6 mois,
 * 22 jours, 7 h 41 min », « 1 heure 30 minutes » — is written here from typed parts.
 */
@Component({
  selector: 'app-dates-durations',
  imports: [ResultRowComponent, TranslocoPipe],
  templateUrl: './durations.component.html',
  styleUrl: './durations.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class DurationsComponent {
  private readonly repository = inject(ToolsRepository);
  private readonly clock = inject(ClockService);
  private readonly transloco = inject(TranslocoService);

  private readonly lang = toSignal(this.transloco.langChanges$, {
    initialValue: this.transloco.getActiveLang(),
  });

  protected readonly from = toolState('dates.from', '');
  protected readonly to = toolState('dates.to', '');
  protected readonly duration = toolState('dates.duration', '');

  protected readonly answer = liveResult(
    () => {
      const request = { from: this.from(), to: this.to(), duration: this.duration() };
      return Object.values(request).every((text) => text.trim() === '') ? undefined : request;
    },
    (request) => this.repository.measureDurations(request),
  );

  protected readonly zone = computed(() => this.answer.value()?.zone ?? null);

  protected readonly measured = computed<Measured | null>(() => {
    const gap = this.answer.value()?.gap;
    return gap?.kind === 'measured' ? gap : null;
  });

  protected readonly gapProblem = computed(() => {
    const gap = this.answer.value()?.gap;
    return gap && gap.kind !== 'measured' ? gap : null;
  });

  protected readonly read = computed(() => {
    const duration = this.answer.value()?.duration;
    return duration?.kind === 'read' ? duration : null;
  });

  protected readonly durationProblem = computed(() => {
    const duration = this.answer.value()?.duration;
    return duration?.kind === 'unreadable' ? duration : null;
  });

  /** « 6 mois, 22 jours, 7 h 41 min ». */
  protected readonly calendarWords = computed(() => {
    const measured = this.measured();
    return measured ? this.spellCalendar(measured.calendar, measured.negative) : '';
  });

  protected readonly elapsedRows = computed(() => {
    const measured = this.measured();
    if (!measured) return [];
    const { elapsed } = measured;
    const minutes = String(elapsed.minutes).padStart(2, '0');
    const say = (key: string, params: Record<string, unknown>) =>
      this.say(`tools.dates.elapsed.${key}`, params);
    return [
      { id: 'days', value: say('days', { days: this.number(elapsed.days), hours: elapsed.hours, minutes }) },
      { id: 'weeks', value: say('weeks', { weeks: this.number(elapsed.weeks), days: elapsed.weekDays }) },
      { id: 'hours', value: say('hours', { hours: this.number(elapsed.totalHours), minutes }) },
      { id: 'minutes', value: this.number(elapsed.totalMinutes) },
      { id: 'iso', value: measured.iso },
    ];
  });

  /** Where a daylight-saving change parts the calendar's count from the clock's. */
  protected readonly transitionNote = computed(() => {
    const measured = this.measured();
    const transition = measured?.transition;
    if (!measured || !transition) return null;
    const hours = (minutes: number) => {
      const shown = String(Math.abs(minutes) % 60).padStart(2, '0');
      return this.say('tools.dates.elapsed.hours', {
        hours: this.number(Math.trunc(Math.abs(minutes) / 60)),
        minutes: shown,
      });
    };
    const size = Math.abs(transition.minutes);
    const difference =
      size % 60 === 0
        ? this.say('tools.dates.transition.hoursApart', { count: size / 60 })
        : this.say('tools.dates.transition.minutesApart', { minutes: size });
    const key = `tools.dates.transition.${transition.forward ? 'forward' : 'backward'}${transition.date ? '' : 'Undated'}`;
    return this.say(key, {
      date: transition.date ? this.dayOfMonth(transition.date) : '',
      wall: hours(transition.wallMinutes),
      elapsed: hours(measured.elapsed.totalMinutes),
      difference,
    });
  });

  /** « 1 heure 30 minutes ». */
  protected readonly durationWords = computed(() => {
    const read = this.read();
    return read ? this.spellParts(read.parts, read.negative) : '';
  });

  protected readonly durationRows = computed(() => {
    const read = this.read();
    if (!read) return [];
    const iso = read.fromWords ? [{ id: 'iso', value: read.iso }] : [];
    const { totals: given } = read;
    const totals = given
      ? (['seconds', 'minutes', 'hours'] as const).map((id) => ({ id, value: this.number(given[id] ?? 0) }))
      : [];
    return [...iso, ...totals];
  });

  readonly result = computed<ToolResult | null>(() => {
    const measured = this.measured();
    const read = this.read();
    const lines = [
      ...(measured ? [`${this.from().trim()} → ${this.to().trim()}`, measured.iso] : []),
      ...(read ? [read.totals ? `${read.iso} = ${read.totals.seconds} s` : read.iso] : []),
    ];
    return lines.length === 0
      ? null
      : {
          title: { key: 'tools.dates.noteDurations' },
          kind: 'snippet',
          language: 'txt',
          content: lines.join('\n'),
        };
  });

  protected fieldName(field: GapField): string {
    return this.say(`tools.dates.fields.${field}`);
  }

  protected toNow(): void {
    this.to.set(wallClock(this.clock.now()));
  }

  protected onFrom(event: Event): void {
    this.from.set((event.target as HTMLInputElement).value);
  }

  protected onTo(event: Event): void {
    this.to.set((event.target as HTMLInputElement).value);
  }

  protected onDuration(event: Event): void {
    this.duration.set((event.target as HTMLInputElement).value);
  }

  private say(key: string, params: Record<string, unknown> = {}): string {
    this.lang();
    return this.transloco.translate(key, params);
  }

  private number(value: number): string {
    return new Intl.NumberFormat(this.lang(), { maximumFractionDigits: 6 }).format(value);
  }

  /** "2026-03-29" as « 29 mars », the date itself and no zone. */
  private dayOfMonth(date: string): string {
    const [year = 0, month = 1, day = 1] = date.split('-').map(Number);
    return new Intl.DateTimeFormat(this.lang(), { day: 'numeric', month: 'long', timeZone: 'UTC' }).format(
      new Date(Date.UTC(year, month - 1, day)),
    );
  }

  private spellCalendar(calendar: CalendarGap, negative: boolean): string {
    const dates = DATE_UNITS.filter((unit) => calendar[unit] > 0).map((unit) =>
      this.say(`tools.dates.calendar.${unit}`, { count: calendar[unit] }),
    );
    const clock = CLOCK_UNITS.filter((unit) => calendar[unit] > 0)
      .map((unit) => this.say(`tools.dates.clock.${unit}`, { value: calendar[unit] }))
      .join(' ');
    const words = [...dates, clock].filter((part) => part !== '').join(', ');
    if (words === '') return this.say('tools.dates.calendar.none');
    return negative ? `−${words}` : words;
  }

  private spellParts(parts: DurationParts, negative: boolean): string {
    const words = PART_UNITS.map((unit) => ({ unit, value: parts[unit] ?? 0 }))
      .filter(({ value }) => value !== 0)
      .map(
        ({ unit, value }) =>
          `${this.number(value)} ${this.say(`tools.dates.units.${unit}`, { count: value })}`,
      )
      .join(' ');
    if (words === '') return this.say('tools.dates.units.none');
    return negative ? `−${words}` : words;
  }
}
