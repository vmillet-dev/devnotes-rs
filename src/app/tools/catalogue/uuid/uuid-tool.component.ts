import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { ToolsRepository } from '@core/data/tools.repository';
import { UuidVersion } from '@core/model/tool-answers.model';
import { liveResult } from '@core/services/tools/live-result';
import { Tool, ToolResult } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import {
  Segment,
  SegmentedChoiceComponent,
} from '@shared/controls/segmented-choice/segmented-choice.component';
import { CopyValueComponent } from '@tools/ui/copy-value/copy-value.component';

const VERSIONS: readonly Segment[] = (['v4', 'v7'] as const).map((id) => ({
  id,
  labelKey: `tools.uuid.versions.${id}`,
}));

@Component({
  selector: 'app-uuid-tool',
  imports: [CopyValueComponent, SegmentedChoiceComponent, TranslocoPipe],
  templateUrl: './uuid-tool.component.html',
  styleUrl: './uuid-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class UuidToolComponent implements Tool {
  private readonly repository = inject(ToolsRepository);

  protected readonly version = toolState<UuidVersion>('uuid.version', 'v4');
  protected readonly count = toolState('uuid.count', 5);
  protected readonly uppercase = toolState('uuid.uppercase', false);
  protected readonly checked = toolState('uuid.checked', '');

  private readonly draw = signal(0);

  protected readonly versions = VERSIONS;

  protected readonly generated = liveResult(
    () => ({ version: this.version(), count: this.count(), uppercase: this.uppercase(), draw: this.draw() }),
    ({ draw: _draw, ...request }) => this.repository.generateUuids(request),
  );

  protected readonly list = computed(() => (this.generated.value() ?? []).join('\n'));

  protected readonly inspection = liveResult(
    () => (this.checked().trim() === '' ? undefined : this.checked()),
    (text) => this.repository.inspectUuid(text),
  );

  /** "2024-09-25 23:05:01.488 UTC": the instant as Rust wrote it, read in any language. */
  protected readonly created = computed(() => {
    const inspection = this.inspection.value();
    const created = inspection?.kind === 'valid' ? inspection.created : null;
    return created ? created.replace('T', ' ').replace('Z', ' UTC') : null;
  });

  readonly result = computed<ToolResult | null>(() => {
    const list = this.list();
    return list
      ? { title: { key: 'tools.uuid.noteTitle' }, kind: 'snippet', language: 'txt', content: list }
      : null;
  });

  clear(): void {
    this.checked.set('');
  }

  protected regenerate(): void {
    this.draw.update((draw) => draw + 1);
  }

  protected onVersion(id: string): void {
    this.version.set(id as UuidVersion);
  }

  protected onCount(event: Event): void {
    const count = Number.parseInt((event.target as HTMLInputElement).value, 10);
    if (Number.isFinite(count)) {
      this.count.set(count);
    }
  }

  protected onUppercase(event: Event): void {
    this.uppercase.set((event.target as HTMLInputElement).checked);
  }

  protected onChecked(event: Event): void {
    this.checked.set((event.target as HTMLInputElement).value);
  }
}
