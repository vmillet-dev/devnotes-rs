import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { ToolsRepository } from '@core/data/tools.repository';
import { UrlScope } from '@core/model/tool-answers.model';
import { liveResult } from '@core/services/tools/live-result';
import { Tool, ToolResult } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import {
  Segment,
  SegmentedChoiceComponent,
} from '@shared/controls/segmented-choice/segmented-choice.component';
import { CopyValueComponent } from '@tools/ui/copy-value/copy-value.component';

type Side = 'decoded' | 'encoded';

const SCOPES: readonly Segment[] = (['component', 'whole'] as const).map((id) => ({
  id,
  labelKey: `tools.url-codec.scopes.${id}`,
}));

/** Both fields are typed into: the one typed in last is the text, the other its translation. */
@Component({
  selector: 'app-url-codec-tool',
  imports: [CopyValueComponent, SegmentedChoiceComponent, TranslocoPipe],
  templateUrl: './url-codec-tool.component.html',
  styleUrl: './url-codec-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class UrlCodecToolComponent implements Tool {
  private readonly repository = inject(ToolsRepository);

  protected readonly text = toolState('url-codec.text', '');
  protected readonly side = toolState<Side>('url-codec.side', 'decoded');
  protected readonly scope = toolState<UrlScope>('url-codec.scope', 'component');

  protected readonly scopes = SCOPES;

  private readonly direction = computed(() =>
    this.side() === 'decoded' ? ('encode' as const) : ('decode' as const),
  );

  protected readonly answer = liveResult(
    () =>
      this.text() === ''
        ? undefined
        : { text: this.text(), direction: this.direction(), scope: this.scope() },
    (request) => this.repository.urlCodec(request),
  );

  /** An answer going the other way is not shown: typed on the other side, it is the reverse. */
  private readonly current = computed(() =>
    this.answer.answered()?.direction === this.direction() ? this.answer.value() : null,
  );

  private readonly translated = computed(() => {
    const answer = this.current();
    return answer?.kind === 'done' ? answer.text : '';
  });

  protected readonly decoded = computed(() => (this.side() === 'decoded' ? this.text() : this.translated()));
  protected readonly encoded = computed(() => (this.side() === 'encoded' ? this.text() : this.translated()));

  protected readonly problem = computed(() => {
    const answer = this.current();
    return answer === null || answer.kind === 'done' ? null : answer;
  });

  readonly result = computed<ToolResult | null>(() => {
    const translated = this.translated();
    if (translated === '') return null;

    const encoding = this.answer.answered()?.direction === 'encode';
    return {
      title: { key: encoding ? 'tools.url-codec.noteEncoded' : 'tools.url-codec.noteDecoded' },
      kind: 'snippet',
      language: 'txt',
      content: translated,
    };
  });

  sample(): void {
    this.side.set('decoded');
    this.text.set('café crème & co/?x=1');
  }

  clear(): void {
    this.text.set('');
  }

  protected onInput(side: Side, event: Event): void {
    this.side.set(side);
    this.text.set((event.target as HTMLTextAreaElement).value);
  }

  protected onScope(id: string): void {
    this.scope.set(id as UrlScope);
  }
}
