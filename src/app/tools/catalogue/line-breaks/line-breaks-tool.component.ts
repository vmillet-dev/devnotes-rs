import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { ToolsRepository } from '@core/data/tools.repository';
import { FinalNewline, LineEnding } from '@core/model/tool-answers.model';
import { ClipboardService } from '@core/services/clipboard/clipboard.service';
import { liveResult } from '@core/services/tools/live-result';
import { Tool, ToolResult } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import {
  Segment,
  SegmentedChoiceComponent,
} from '@shared/controls/segmented-choice/segmented-choice.component';
import { CopyValueComponent } from '@tools/ui/copy-value/copy-value.component';
import { LineViewComponent } from './line-view.component';

type EndingChoice = LineEnding | 'keep';

const ENDINGS: readonly Segment[] = (['keep', 'lf', 'crlf', 'cr'] as const).map((id) => ({
  id,
  labelKey: `tools.line-breaks.endings.${id}`,
}));

const FINALS: readonly Segment[] = (['keep', 'add', 'remove'] as const).map((id) => ({
  id,
  labelKey: `tools.line-breaks.finals.${id}`,
}));

/**
 * ⚠️ No `<textarea>` for the input: its value turns every CRLF and CR into LF, which is the
 * very thing this tool is asked about. A paste is read raw, and the text drawn with its endings.
 */
@Component({
  selector: 'app-line-breaks-tool',
  imports: [CopyValueComponent, LineViewComponent, SegmentedChoiceComponent, TranslocoPipe],
  templateUrl: './line-breaks-tool.component.html',
  styleUrl: './line-breaks-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class LineBreaksToolComponent implements Tool {
  private readonly repository = inject(ToolsRepository);
  private readonly clipboard = inject(ClipboardService);

  protected readonly text = toolState('line-breaks.text', '');
  protected readonly ending = toolState<EndingChoice>('line-breaks.ending', 'keep');
  protected readonly trim = toolState('line-breaks.trim', true);
  protected readonly final = toolState<FinalNewline>('line-breaks.final', 'keep');

  protected readonly endings = ENDINGS;
  protected readonly finals = FINALS;

  protected readonly answer = liveResult(
    () => {
      const ending = this.ending();
      return this.text() === ''
        ? undefined
        : {
            text: this.text(),
            ending: ending === 'keep' ? null : ending,
            trimTrailing: this.trim(),
            finalNewline: this.final(),
          };
    },
    (request) => this.repository.fixLineBreaks(request),
  );

  /** More than one kind of ending in one text: the case this tool exists for. */
  protected readonly mixed = computed(() => {
    const found = this.answer.value()?.found;
    return found ? [found.lf, found.crlf, found.cr].filter((count) => count > 0).length > 1 : false;
  });

  readonly result = computed<ToolResult | null>(() => {
    const answer = this.answer.value();
    return answer?.text
      ? {
          title: { key: 'tools.line-breaks.noteTitle' },
          kind: 'snippet',
          language: 'txt',
          content: answer.text,
        }
      : null;
  });

  clear(): void {
    this.text.set('');
  }

  protected onPaste(event: ClipboardEvent): void {
    const text = event.clipboardData?.getData('text/plain');
    if (text === undefined) return;

    event.preventDefault();
    this.text.set(text);
  }

  protected async pasteFromClipboard(): Promise<void> {
    const text = await this.clipboard.paste();
    if (text !== '') {
      this.text.set(text);
    }
  }

  protected onEnding(id: string): void {
    this.ending.set(id as EndingChoice);
  }

  protected onFinal(id: string): void {
    this.final.set(id as FinalNewline);
  }

  protected onTrim(event: Event): void {
    this.trim.set((event.target as HTMLInputElement).checked);
  }
}
