import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import { ToolsRepository } from '@core/data/tools.repository';
import { CaseConversion, TextCase, TitleLanguage } from '@core/model/tool-answers.model';
import { liveResult } from '@core/services/tools/live-result';
import { Tool, ToolResult } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import {
  Segment,
  SegmentedChoiceComponent,
} from '@shared/controls/segmented-choice/segmented-choice.component';
import { ResultRowComponent } from '@tools/ui/result-row/result-row.component';

/** Each case spelled in itself: a note keeps them in no one language. */
const CASE_NAMES: Record<TextCase, string> = {
  camel: 'camelCase',
  pascal: 'PascalCase',
  snake: 'snake_case',
  kebab: 'kebab-case',
  constant: 'CONSTANT_CASE',
  title: 'Title Case',
  sentence: 'Sentence case',
  dot: 'dot.case',
  path: 'path/case',
  train: 'Train-Case',
  lower: 'lowercase',
  upper: 'UPPERCASE',
  flat: 'flatcase',
};

/** On screen, the two whose name is a word of the language rather than an example of itself. */
const SHOWN_NAME_KEYS: Partial<Record<TextCase, string>> = {
  upper: 'tools.case.upper',
  lower: 'tools.case.lower',
};

const LANGUAGES: readonly Segment[] = (['english', 'french'] as const).map((id) => ({
  id,
  labelKey: `tools.case.${id}`,
}));

@Component({
  selector: 'app-case-tool',
  imports: [ResultRowComponent, SegmentedChoiceComponent, TranslocoPipe],
  templateUrl: './case-tool.component.html',
  styleUrl: './case-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CaseToolComponent implements Tool {
  private readonly transloco = inject(TranslocoService);
  private readonly repository = inject(ToolsRepository);

  protected readonly text = toolState('case.text', '');
  protected readonly stripAccents = toolState('case.stripAccents', true);
  protected readonly perLine = toolState('case.perLine', true);
  protected readonly titleLanguage = toolState<TitleLanguage>('case.titleLanguage', 'english');

  protected readonly answer = liveResult(
    () =>
      this.text().trim() === ''
        ? undefined
        : {
            text: this.text(),
            stripAccents: this.stripAccents(),
            perLine: this.perLine(),
            titleCaseLanguage: this.titleLanguage(),
          },
    (request) => this.repository.convertCase(request),
  );

  protected readonly languages = LANGUAGES;

  protected readonly words = computed(() => this.answer.value()?.words ?? []);
  protected readonly forCode = computed(() => this.rowsOf('code'));
  protected readonly forText = computed(() => this.rowsOf('text'));

  readonly result = computed<ToolResult | null>(() => {
    const conversions = this.answer.value()?.conversions;
    if (!conversions?.length) return null;

    const width = Math.max(...conversions.map(({ case: kind }) => CASE_NAMES[kind].length));
    return {
      title: {
        key: 'tools.case.noteTitle',
        // An answer is always paired with the request it answers.
        params: { text: this.answer.answered()!.text.trim().split('\n')[0]!.trim().slice(0, 40) },
      },
      kind: 'snippet',
      language: 'txt',
      content: conversions
        .map(
          ({ case: kind, value }) =>
            // A list converted line by line keeps its lines under the first, past the names.
            `${CASE_NAMES[kind].padEnd(width)}  ${value.split('\n').join(`\n${' '.repeat(width + 2)}`)}`,
        )
        .join('\n'),
    };
  });

  sample(): void {
    this.text.set(this.transloco.translate('tools.case.sample'));
  }

  clear(): void {
    this.text.set('');
  }

  protected shownName(kind: TextCase): string {
    const key = SHOWN_NAME_KEYS[kind];
    return key ? this.transloco.translate(key) : CASE_NAMES[kind];
  }

  protected onInput(event: Event): void {
    this.text.set((event.target as HTMLTextAreaElement).value);
  }

  protected onStripAccents(event: Event): void {
    this.stripAccents.set((event.target as HTMLInputElement).checked);
  }

  protected onPerLine(event: Event): void {
    this.perLine.set((event.target as HTMLInputElement).checked);
  }

  protected onTitleLanguage(language: string): void {
    this.titleLanguage.set(language as TitleLanguage);
  }

  private rowsOf(group: CaseConversion['group']): readonly CaseConversion[] {
    return (this.answer.value()?.conversions ?? []).filter((conversion) => conversion.group === group);
  }
}
