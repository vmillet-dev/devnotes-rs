import { ChangeDetectionStrategy, Component, computed, viewChild } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { Tool, ToolAction, ToolResult } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import {
  Segment,
  SegmentedChoiceComponent,
} from '@shared/controls/segmented-choice/segmented-choice.component';
import { StructuredDiffComponent } from './structured/structured-diff.component';
import { TextDiffComponent } from './text/text-diff.component';

type Mode = 'text' | 'structured';

const MODES: readonly Segment[] = (['text', 'structured'] as const).map((id) => ({
  id,
  labelKey: `tools.diff.modes.${id}`,
}));

/** Two modes over the same two texts; the frame talks to whichever is on screen. */
@Component({
  selector: 'app-diff-tool',
  imports: [SegmentedChoiceComponent, StructuredDiffComponent, TextDiffComponent, TranslocoPipe],
  templateUrl: './diff-tool.component.html',
  styleUrl: './diff-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class DiffToolComponent implements Tool {
  protected readonly mode = toolState<Mode>('diff.mode', 'text');
  protected readonly modes = MODES;

  private readonly text = viewChild(TextDiffComponent);
  private readonly structured = viewChild(StructuredDiffComponent);
  private readonly shown = computed<Tool | undefined>(() =>
    this.mode() === 'text' ? this.text() : this.structured(),
  );

  readonly result = computed<ToolResult | null>(() => this.shown()?.result() ?? null);
  readonly actions = computed<readonly ToolAction[]>(() => this.shown()?.actions?.() ?? []);

  sample(): void {
    this.shown()?.sample?.();
  }

  clear(): void {
    this.shown()?.clear();
  }

  protected onMode(id: string): void {
    this.mode.set(id as Mode);
  }
}
