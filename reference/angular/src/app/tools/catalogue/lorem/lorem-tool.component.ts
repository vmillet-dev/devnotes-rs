import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { ToolsRepository } from '@core/data/tools.repository';
import { LoremUnit } from '@core/model/tool-answers.model';
import { liveResult } from '@core/services/tools/live-result';
import { Tool, ToolResult } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import {
  Segment,
  SegmentedChoiceComponent,
} from '@shared/controls/segmented-choice/segmented-choice.component';
import { CopyValueComponent } from '@tools/ui/copy-value/copy-value.component';

const UNITS: readonly Segment[] = (['words', 'sentences', 'paragraphs'] as const).map((id) => ({
  id,
  labelKey: `tools.lorem.units.${id}`,
}));

@Component({
  selector: 'app-lorem-tool',
  imports: [CopyValueComponent, SegmentedChoiceComponent, TranslocoPipe],
  templateUrl: './lorem-tool.component.html',
  styleUrl: './lorem-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class LoremToolComponent implements Tool {
  private readonly repository = inject(ToolsRepository);

  protected readonly unit = toolState<LoremUnit>('lorem.unit', 'paragraphs');
  protected readonly count = toolState('lorem.count', 3);
  protected readonly opening = toolState('lorem.opening', true);

  private readonly draw = signal(0);

  protected readonly units = UNITS;

  protected readonly answer = liveResult(
    () => ({
      unit: this.unit(),
      count: this.count(),
      opening: this.opening(),
      seed: null,
      draw: this.draw(),
    }),
    ({ draw: _draw, ...request }) => this.repository.lorem(request),
  );

  protected readonly paragraphs = computed(() => (this.answer.value()?.text ?? '').split('\n\n'));

  /** The count asked, when Rust held it to its bound: said rather than silently lowered. */
  protected readonly heldTo = computed(() => {
    const answer = this.answer.value();
    const asked = this.answer.answered();
    return answer && asked && asked.count > answer.atMost ? answer.atMost : null;
  });

  readonly result = computed<ToolResult | null>(() => {
    const text = this.answer.value()?.text;
    return text
      ? { title: { key: 'tools.lorem.noteTitle' }, kind: 'snippet', language: 'txt', content: text }
      : null;
  });

  /** Three paragraphs, newly drawn. */
  sample(): void {
    this.unit.set('paragraphs');
    this.count.set(3);
    this.opening.set(true);
    this.draw.update((draw) => draw + 1);
  }

  /** Nothing typed to empty: Vider puts the options back as they were. */
  clear(): void {
    this.unit.set('paragraphs');
    this.count.set(3);
    this.opening.set(true);
  }

  protected regenerate(): void {
    this.draw.update((draw) => draw + 1);
  }

  protected onUnit(id: string): void {
    this.unit.set(id as LoremUnit);
  }

  protected onCount(event: Event): void {
    const count = Number.parseInt((event.target as HTMLInputElement).value, 10);
    if (Number.isFinite(count)) {
      this.count.set(count);
    }
  }

  protected onOpening(event: Event): void {
    this.opening.set((event.target as HTMLInputElement).checked);
  }
}
