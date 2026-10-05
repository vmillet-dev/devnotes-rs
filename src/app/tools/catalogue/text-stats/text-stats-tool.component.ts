import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import { ToolsRepository } from '@core/data/tools.repository';
import { TextStats } from '@core/model/tool-answers.model';
import { liveResult } from '@core/services/tools/live-result';
import { Tool, ToolResult } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import { CopyValueComponent } from '@tools/ui/copy-value/copy-value.component';

type Count = keyof Pick<
  TextStats,
  | 'characters'
  | 'nonWhitespace'
  | 'words'
  | 'lines'
  | 'nonEmptyLines'
  | 'paragraphs'
  | 'codePoints'
  | 'utf16Units'
  | 'utf8Bytes'
>;

const COUNTS: readonly Count[] = [
  'characters',
  'nonWhitespace',
  'words',
  'lines',
  'nonEmptyLines',
  'paragraphs',
  'codePoints',
  'utf16Units',
  'utf8Bytes',
];

/** What a reader counts and what a program does, both Rust's; the page draws the bars. */
@Component({
  selector: 'app-text-stats-tool',
  imports: [CopyValueComponent, TranslocoPipe],
  templateUrl: './text-stats-tool.component.html',
  styleUrl: './text-stats-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TextStatsToolComponent implements Tool {
  private readonly repository = inject(ToolsRepository);
  private readonly transloco = inject(TranslocoService);

  protected readonly text = toolState('text-stats.text', '');
  protected readonly foldCase = toolState('text-stats.foldCase', false);
  protected readonly countWhitespace = toolState('text-stats.countWhitespace', false);

  protected readonly counts = COUNTS;

  private readonly translation = toSignal(this.transloco.selectTranslation());

  protected readonly answer = liveResult(
    () =>
      this.text() === ''
        ? undefined
        : { text: this.text(), foldCase: this.foldCase(), countWhitespace: this.countWhitespace() },
    (request) => this.repository.textStats(request),
  );

  /** The widest bar is the most frequent character. */
  protected readonly frequencies = computed(() => {
    const frequencies = this.answer.value()?.frequencies ?? [];
    const most = Math.max(1, ...frequencies.map((frequency) => frequency.count));
    return frequencies.map((frequency) => ({
      ...frequency,
      width: (frequency.count / most) * 100,
      shown: (frequency.share ?? 0).toFixed(1),
    }));
  });

  readonly result = computed<ToolResult | null>(() => {
    this.translation();
    const stats = this.answer.value();
    if (!stats) return null;
    const name = (count: Count) => this.transloco.translate<string>(`tools.text-stats.rows.${count}`);
    const width = Math.max(...COUNTS.map((count) => name(count).length));
    return {
      title: { key: 'tools.text-stats.noteTitle' },
      kind: 'snippet',
      language: 'txt',
      content: COUNTS.map((count) => `${name(count).padEnd(width)}  ${stats[count]}`).join('\n'),
    };
  });

  sample(): void {
    this.text.set(this.transloco.translate('tools.text-stats.sample'));
  }

  clear(): void {
    this.text.set('');
  }

  protected onInput(event: Event): void {
    this.text.set((event.target as HTMLTextAreaElement).value);
  }

  protected onFoldCase(event: Event): void {
    this.foldCase.set((event.target as HTMLInputElement).checked);
  }

  protected onCountWhitespace(event: Event): void {
    this.countWhitespace.set((event.target as HTMLInputElement).checked);
  }
}
