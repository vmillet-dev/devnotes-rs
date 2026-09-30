import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import { ToolsRepository } from '@core/data/tools.repository';
import { FakeColumn, FakeLocale } from '@core/model/tool-answers.model';
import { ClipboardService } from '@core/services/clipboard/clipboard.service';
import { debounced } from '@core/services/time/debounce';
import { liveResult } from '@core/services/tools/live-result';
import { Tool, ToolResult } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import {
  Segment,
  SegmentedChoiceComponent,
} from '@shared/controls/segmented-choice/segmented-choice.component';
import { CopyValueComponent } from '@tools/ui/copy-value/copy-value.component';

type Format = 'csv' | 'json';

const COLUMNS: readonly FakeColumn[] = ['name', 'email', 'phone', 'address', 'iban', 'card'];

const LOCALES: readonly Segment[] = (['fr', 'en'] as const).map((id) => ({
  id,
  labelKey: `tools.fake-data.locales.${id}`,
}));

const FORMATS: readonly Segment[] = (['csv', 'json'] as const).map((id) => ({
  id,
  labelKey: `tools.fake-data.formats.${id}`,
}));

const FEEDBACK_MS = 2000;

/** Every row is Rust's, drawn from a seed; the page lays them out and copies them. */
@Component({
  selector: 'app-fake-data-tool',
  imports: [CopyValueComponent, SegmentedChoiceComponent, TranslocoPipe],
  templateUrl: './fake-data-tool.component.html',
  styleUrl: './fake-data-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class FakeDataToolComponent implements Tool {
  private readonly repository = inject(ToolsRepository);
  private readonly clipboard = inject(ClipboardService);

  protected readonly columns = toolState<readonly FakeColumn[]>('fake-data.columns', [
    'name',
    'email',
    'phone',
    'address',
  ]);
  protected readonly count = toolState('fake-data.count', 10);
  /** The interface's language to begin with; a choice of its own after that. */
  protected readonly locale = toolState<FakeLocale>(
    'fake-data.locale',
    inject(TranslocoService).getActiveLang() === 'en' ? 'en' : 'fr',
  );
  protected readonly seed = toolState<number | null>('fake-data.seed', null);
  protected readonly format = toolState<Format>('fake-data.format', 'csv');

  private readonly draw = signal(0);
  /** The cell just copied, lit a moment. */
  protected readonly copiedCell = signal<string | null>(null);
  private readonly settleCopied = debounced<void>(() => this.copiedCell.set(null), FEEDBACK_MS);

  protected readonly allColumns = COLUMNS;
  protected readonly locales = LOCALES;
  protected readonly formats = FORMATS;

  protected readonly answer = liveResult(
    () =>
      this.columns().length === 0
        ? undefined
        : {
            columns: [...this.columns()],
            count: this.count(),
            locale: this.locale(),
            seed: this.seed(),
            draw: this.draw(),
          },
    ({ draw: _draw, ...request }) => this.repository.fakeData(request),
  );

  protected readonly exported = computed(() => {
    const answer = this.answer.value();
    return answer ? (this.format() === 'csv' ? answer.csv : answer.json) : '';
  });

  readonly result = computed<ToolResult | null>(() => {
    const answer = this.answer.value();
    return answer && answer.rows.length > 0
      ? {
          title: { key: 'tools.fake-data.noteTitle', params: { count: answer.rows.length } },
          kind: 'snippet',
          language: this.format() === 'csv' ? 'txt' : 'json',
          content: this.exported(),
        }
      : null;
  });

  clear(): void {
    this.seed.set(null);
    this.draw.update((draw) => draw + 1);
  }

  protected chosen(column: FakeColumn): boolean {
    return this.columns().includes(column);
  }

  protected onColumn(column: FakeColumn, event: Event): void {
    const checked = (event.target as HTMLInputElement).checked;
    this.columns.update((columns) =>
      checked
        ? COLUMNS.filter((known) => known === column || columns.includes(known))
        : columns.filter((kept) => kept !== column),
    );
  }

  protected onCount(event: Event): void {
    const count = Number.parseInt((event.target as HTMLInputElement).value, 10);
    if (Number.isFinite(count)) this.count.set(Math.min(Math.max(count, 1), 100));
  }

  protected onLocale(id: string): void {
    this.locale.set(id as FakeLocale);
  }

  protected onFormat(id: string): void {
    this.format.set(id as Format);
  }

  protected onSeed(event: Event): void {
    const value = (event.target as HTMLInputElement).value.trim();
    const seed = Number.parseInt(value, 10);
    this.seed.set(value === '' || !Number.isFinite(seed) ? null : Math.max(0, Math.min(seed, 4_294_967_295)));
  }

  protected keepSeed(seed: number): void {
    this.seed.set(seed);
  }

  protected regenerate(): void {
    this.draw.update((draw) => draw + 1);
  }

  protected async copyCell(key: string, value: string): Promise<void> {
    if (!(await this.clipboard.copy(value))) return;
    this.copiedCell.set(key);
    this.settleCopied();
  }
}
