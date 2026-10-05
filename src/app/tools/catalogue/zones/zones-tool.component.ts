import {
  ChangeDetectionStrategy,
  Component,
  computed,
  effect,
  inject,
  signal,
  untracked,
} from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { ToolsRepository } from '@core/data/tools.repository';
import { ZoneTime, ZonesAnswer } from '@core/model/tool-answers.model';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { SettingsStore } from '@core/services/settings/settings.store';
import { liveResult } from '@core/services/tools/live-result';
import { Tool, ToolResult } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import { CopyValueComponent } from '@tools/ui/copy-value/copy-value.component';

type Placed = Extract<ZonesAnswer, { kind: 'placed' }>;

/** A time typed in one zone, placed in each zone of a list; the database and every rule are Rust's. */
@Component({
  selector: 'app-zones-tool',
  imports: [CopyValueComponent, TranslocoPipe],
  templateUrl: './zones-tool.component.html',
  styleUrl: './zones-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ZonesToolComponent implements Tool {
  private readonly repository = inject(ToolsRepository);
  private readonly notifier = inject(ErrorNotifier);
  private readonly settings = inject(SettingsStore);

  protected readonly text = toolState('zones.text', '');
  /** `null` is the machine's zone. */
  protected readonly from = toolState<string | null>('zones.from', null);
  /** Kept with the application's preferences, so the list is found again on the next launch. */
  protected readonly zones = this.settings.timeZones;
  protected readonly later = toolState('zones.later', false);

  protected readonly query = signal('');

  protected readonly answer = liveResult(
    () =>
      this.text().trim() === ''
        ? undefined
        : { text: this.text(), from: this.from(), zones: [...this.zones()], later: this.later() },
    (request) => this.repository.placeInZones(request),
  );

  protected readonly found = liveResult(
    () => (this.query().trim() === '' ? undefined : this.query()),
    (query) => this.repository.searchTimeZones(query),
  );

  protected readonly placed = computed<Placed | null>(() => {
    const answer = this.answer.value();
    return answer?.kind === 'placed' ? answer : null;
  });

  protected readonly problem = computed(() => {
    const answer = this.answer.value();
    return answer === null || answer.kind === 'placed' ? null : answer;
  });

  readonly result = computed<ToolResult | null>(() => {
    const placed = this.placed();
    const asked = this.answer.answered();
    return placed && asked && placed.times.length > 0
      ? {
          title: { key: 'tools.zones.noteTitle', params: { time: asked.text.trim() } },
          kind: 'snippet',
          language: 'txt',
          content: placed.times
            .map((time) =>
              [
                time.zone.padEnd(24),
                `${time.date} ${time.time}`,
                `UTC${time.offset}`,
                time.abbreviation ?? '',
              ]
                .join('  ')
                .trimEnd(),
            )
            .join('\n'),
        }
      : null;
  });

  constructor() {
    // A zone the database does not know leaves the list, once said.
    effect(() => {
      const unknown = this.placed()?.unknown ?? [];
      if (unknown.length > 0) {
        untracked(() => this.zones.write(this.zones().filter((zone) => !unknown.includes(zone))));
      }
    });
  }

  sample(): void {
    this.text.set('2026-03-29 09:00');
    this.later.set(false);
  }

  clear(): void {
    this.text.set('');
    this.later.set(false);
    this.query.set('');
  }

  /** A zone of the list can go; the machine's and the one typed in are there whatever the list says. */
  protected removable(time: ZoneTime): boolean {
    return !time.local && !time.source && this.zones().includes(time.zone);
  }

  protected async takeNow(): Promise<void> {
    const now = await this.notifier.attempt('errors.toolFailed', () =>
      this.repository.timeInZone(this.from()),
    );
    if (now !== null) {
      this.later.set(false);
      this.text.set(now);
    }
  }

  protected typeIn(zone: string): void {
    this.from.set(zone);
    this.later.set(false);
  }

  protected add(zone: string): void {
    if (!this.zones().includes(zone)) {
      this.zones.write([...this.zones(), zone]);
    }
    this.query.set('');
  }

  protected remove(zone: string): void {
    this.zones.write(this.zones().filter((listed) => listed !== zone));
  }

  protected onInput(event: Event): void {
    this.text.set((event.target as HTMLInputElement).value);
    this.later.set(false);
  }

  protected onQuery(event: Event): void {
    this.query.set((event.target as HTMLInputElement).value);
  }

  protected onReading(later: boolean): void {
    this.later.set(later);
  }
}
