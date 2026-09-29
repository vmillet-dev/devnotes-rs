import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { ToolsRepository } from '@core/data/tools.repository';
import { SlugSeparator } from '@core/model/tool-answers.model';
import { liveResult } from '@core/services/tools/live-result';
import { Tool, ToolResult } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import {
  Segment,
  SegmentedChoiceComponent,
} from '@shared/controls/segmented-choice/segmented-choice.component';
import { OutputRowComponent } from '@tools/ui/output-row/output-row.component';

const SEPARATORS: readonly Segment[] = (['dash', 'underscore', 'dot'] as const).map((id) => ({
  id,
  labelKey: `tools.slug.separators.${id}`,
}));

@Component({
  selector: 'app-slug-tool',
  imports: [OutputRowComponent, SegmentedChoiceComponent, TranslocoPipe],
  templateUrl: './slug-tool.component.html',
  styleUrl: './slug-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SlugToolComponent implements Tool {
  private readonly repository = inject(ToolsRepository);

  protected readonly text = toolState('slug.text', '');
  protected readonly separator = toolState<SlugSeparator>('slug.separator', 'dash');
  protected readonly lowercase = toolState('slug.lowercase', true);

  protected readonly separators = SEPARATORS;

  protected readonly slug = liveResult(
    () =>
      this.text().trim() === ''
        ? undefined
        : { text: this.text(), separator: this.separator(), lowercase: this.lowercase() },
    (request) => this.repository.slugify(request),
  );

  readonly result = computed<ToolResult | null>(() => {
    const slug = this.slug.value();
    return slug
      ? {
          title: {
            key: 'tools.slug.noteTitle',
            params: { text: (this.slug.answered()?.text ?? '').trim().slice(0, 40) },
          },
          kind: 'snippet',
          language: 'txt',
          content: slug,
        }
      : null;
  });

  clear(): void {
    this.text.set('');
  }

  protected onInput(event: Event): void {
    this.text.set((event.target as HTMLTextAreaElement).value);
  }

  protected onLowercase(event: Event): void {
    this.lowercase.set((event.target as HTMLInputElement).checked);
  }

  protected onSeparator(id: string): void {
    this.separator.set(id as SlugSeparator);
  }
}
