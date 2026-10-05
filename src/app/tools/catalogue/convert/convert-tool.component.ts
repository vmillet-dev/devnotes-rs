import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { ToolsRepository } from '@core/data/tools.repository';
import { LanguageTag } from '@core/model/language.model';
import { DataFormat } from '@core/model/tool-answers.model';
import { liveResult } from '@core/services/tools/live-result';
import { Tool, ToolAction, ToolResult } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import {
  Segment,
  SegmentedChoiceComponent,
} from '@shared/controls/segmented-choice/segmented-choice.component';
import { CopyValueComponent } from '@tools/ui/copy-value/copy-value.component';

const FORMATS: readonly DataFormat[] = ['json', 'toml', 'xml', 'yaml'];

const SEGMENTS: readonly Segment[] = FORMATS.map((id) => ({ id, labelKey: `tools.convert.formats.${id}` }));

/** What a note keeps the result as, so it is highlighted in its own language. */
const LANGUAGES: Record<DataFormat, LanguageTag> = { json: 'json', toml: 'toml', xml: 'xml', yaml: 'yml' };

const SAMPLE = `{
  "name": "DevNotes",
  "version": "0.9.2",
  "features": ["notes", "tools"],
  "window": { "width": 1440, "height": 900, "maximised": false }
}`;

/** Any format to any other, through one value in Rust; the conventions are said beside the result. */
@Component({
  selector: 'app-convert-tool',
  imports: [CopyValueComponent, SegmentedChoiceComponent, TranslocoPipe],
  templateUrl: './convert-tool.component.html',
  styleUrl: './convert-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ConvertToolComponent implements Tool {
  private readonly repository = inject(ToolsRepository);

  protected readonly text = toolState('convert.text', '');
  protected readonly from = toolState<DataFormat>('convert.from', 'json');
  protected readonly to = toolState<DataFormat>('convert.to', 'yaml');

  protected readonly formats = SEGMENTS;

  protected readonly answer = liveResult(
    () => (this.text().trim() === '' ? undefined : { text: this.text(), from: this.from(), to: this.to() }),
    (request) => this.repository.convert(request),
  );

  protected readonly converted = computed(() => {
    const answer = this.answer.value();
    return answer?.kind === 'converted' ? answer.text : null;
  });

  protected readonly unreadable = computed(() => {
    const [answer, asked] = [this.answer.value(), this.answer.answered()];
    return answer?.kind === 'unreadable' && asked ? { ...answer, format: asked.from } : null;
  });

  protected readonly impossible = computed(() => {
    const answer = this.answer.value();
    return answer?.kind === 'impossible' ? answer : null;
  });

  /** A convention is said wherever the format it concerns is on either side. */
  protected readonly conventions = computed(() =>
    (['xml', 'toml'] as const).filter((format) => this.from() === format || this.to() === format),
  );

  readonly actions = computed<readonly ToolAction[]>(() => [
    { id: 'swap', labelKey: 'tools.convert.swap', disabled: false, run: () => this.swap() },
  ]);

  readonly result = computed<ToolResult | null>(() => {
    const [converted, asked] = [this.converted(), this.answer.answered()];
    return converted && asked
      ? {
          title: {
            key: 'tools.convert.noteTitle',
            params: { from: asked.from.toUpperCase(), to: asked.to.toUpperCase() },
          },
          kind: 'snippet',
          language: LANGUAGES[asked.to],
          content: converted,
        }
      : null;
  });

  sample(): void {
    this.from.set('json');
    this.to.set('yaml');
    this.text.set(SAMPLE);
  }

  clear(): void {
    this.text.set('');
  }

  /** The result becomes what is converted back: the way to check a round trip. */
  protected swap(): void {
    const [converted, asked] = [this.converted(), this.answer.answered()];
    const [from, to] = converted !== null && asked ? [asked.from, asked.to] : [this.from(), this.to()];
    this.from.set(to);
    this.to.set(from);
    if (converted !== null) {
      this.text.set(converted);
    }
  }

  protected onInput(event: Event): void {
    this.text.set((event.target as HTMLTextAreaElement).value);
  }

  protected onFrom(id: string): void {
    this.from.set(id as DataFormat);
  }

  protected onTo(id: string): void {
    this.to.set(id as DataFormat);
  }
}
