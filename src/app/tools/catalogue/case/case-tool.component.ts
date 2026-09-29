import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { ToolsRepository } from '@core/data/tools.repository';
import { TextCase } from '@core/model/tool-answers.model';
import { liveResult } from '@core/services/tools/live-result';
import { Tool, ToolResult } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import { OutputRowComponent } from '@tools/ui/output-row/output-row.component';

/** Each case spelled in itself: a note keeps them in no one language. */
const CASE_NAMES: Record<TextCase, string> = {
  camel: 'camelCase',
  pascal: 'PascalCase',
  snake: 'snake_case',
  kebab: 'kebab-case',
  constant: 'CONSTANT_CASE',
  title: 'Title Case',
  sentence: 'Sentence case',
};

@Component({
  selector: 'app-case-tool',
  imports: [OutputRowComponent, TranslocoPipe],
  templateUrl: './case-tool.component.html',
  styleUrl: './case-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CaseToolComponent implements Tool {
  private readonly repository = inject(ToolsRepository);

  protected readonly text = toolState('case.text', '');

  protected readonly conversions = liveResult(
    () => (this.text().trim() === '' ? undefined : this.text()),
    (text) => this.repository.convertCase(text),
  );

  protected readonly caseNames = CASE_NAMES;

  readonly result = computed<ToolResult | null>(() => {
    const conversions = this.conversions.value();
    if (!conversions?.length) return null;

    const width = Math.max(...conversions.map(({ case: kind }) => CASE_NAMES[kind].length));
    return {
      title: { key: 'tools.case.noteTitle', params: { text: this.text().trim().slice(0, 40) } },
      kind: 'snippet',
      language: 'txt',
      content: conversions
        .map(({ case: kind, value }) => `${CASE_NAMES[kind].padEnd(width)}  ${value}`)
        .join('\n'),
    };
  });

  clear(): void {
    this.text.set('');
  }

  protected onInput(event: Event): void {
    this.text.set((event.target as HTMLTextAreaElement).value);
  }
}
