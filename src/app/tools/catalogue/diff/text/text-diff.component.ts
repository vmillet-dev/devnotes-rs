import { ChangeDetectionStrategy, Component, computed, inject, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { ToolsRepository } from '@core/data/tools.repository';
import { Granularity } from '@core/model/tool-answers.model';
import { ClipboardService } from '@core/services/clipboard/clipboard.service';
import { liveResult } from '@core/services/tools/live-result';
import { Tool, ToolAction, ToolResult } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import {
  Segment,
  SegmentedChoiceComponent,
} from '@shared/controls/segmented-choice/segmented-choice.component';

type Layout = 'split' | 'unified';
type Option = 'trailing' | 'whitespace' | 'case' | 'endings';

const GRANULARITIES: readonly Segment[] = (['lines', 'words', 'characters'] as const).map((id) => ({
  id,
  labelKey: `tools.diff.text.granularities.${id}`,
}));

const LAYOUTS: readonly Segment[] = (['split', 'unified'] as const).map((id) => ({
  id,
  labelKey: `tools.diff.text.layouts.${id}`,
}));

const OPTIONS: readonly Option[] = ['trailing', 'whitespace', 'case', 'endings'];

const MARKS = { same: '', added: '+', removed: '−', modified: '~' } as const;

const SAMPLE = {
  a: `server {
    listen 80;
    server_name devnotes.fr;
    root /var/www/devnotes;
    gzip off;
}
`,
  b: `server {
    listen 443 ssl;
    server_name devnotes.fr www.devnotes.fr;
    root /var/www/devnotes;
    gzip on;
    ssl_certificate /etc/ssl/devnotes.pem;
}
`,
};

/** A line of `diff -u`, by how it starts. */
function kindOf(line: string): 'file' | 'hunk' | 'added' | 'removed' | 'same' {
  if (line.startsWith('+++') || line.startsWith('---')) return 'file';
  if (line.startsWith('@@')) return 'hunk';
  if (line.startsWith('+')) return 'added';
  if (line.startsWith('-')) return 'removed';
  return 'same';
}

/** Shares its two texts with the structured mode: switching mode compares the same documents. */
@Component({
  selector: 'app-text-diff',
  imports: [SegmentedChoiceComponent, TranslocoPipe],
  templateUrl: './text-diff.component.html',
  styleUrl: './text-diff.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TextDiffComponent implements Tool {
  private readonly repository = inject(ToolsRepository);
  private readonly clipboard = inject(ClipboardService);

  readonly asValues = output<void>();

  protected readonly a = toolState('diff.a', '');
  protected readonly b = toolState('diff.b', '');
  protected readonly aName = toolState<string | null>('diff.aName', null);
  protected readonly bName = toolState<string | null>('diff.bName', null);
  protected readonly granularity = toolState<Granularity>('diff.granularity', 'lines');
  protected readonly layout = toolState<Layout>('diff.textLayout', 'split');
  private readonly ignored = toolState<ReadonlySet<Option>>('diff.ignored', new Set(['endings']));

  protected readonly granularities = GRANULARITIES;
  protected readonly layouts = LAYOUTS;
  protected readonly options = OPTIONS;
  protected readonly marks = MARKS;

  protected readonly answer = liveResult(
    () => {
      const ignored = this.ignored();
      return this.a() === '' && this.b() === ''
        ? undefined
        : {
            a: this.a(),
            b: this.b(),
            granularity: this.granularity(),
            ignoreTrailingWhitespace: ignored.has('trailing'),
            ignoreAllWhitespace: ignored.has('whitespace'),
            ignoreCase: ignored.has('case'),
            ignoreLineEndings: ignored.has('endings'),
          };
    },
    (request) => this.repository.diffText(request),
    250,
  );

  /** The unified diff, once there is a difference to write. */
  private readonly unified = computed(() => {
    const answer = this.answer.value();
    return answer && !answer.identical ? answer.unified : null;
  });

  protected readonly unifiedLines = computed(() =>
    (this.unified() ?? '')
      .replace(/\n$/, '')
      .split('\n')
      .map((text) => ({ text, kind: kindOf(text) })),
  );

  readonly actions = computed<readonly ToolAction[]>(() => [
    { id: 'swap', labelKey: 'tools.diff.swap', disabled: false, run: () => this.swap() },
    {
      id: 'copy-unified',
      labelKey: 'tools.diff.text.copyUnified',
      disabled: this.unified() === null,
      run: () => void this.copyUnified(),
    },
  ]);

  readonly result = computed<ToolResult | null>(() => {
    const unified = this.unified();
    return unified
      ? {
          title: {
            key: 'tools.diff.text.noteTitle',
            params: { a: this.aName() ?? 'A', b: this.bName() ?? 'B' },
          },
          kind: 'snippet',
          language: 'txt',
          content: unified,
        }
      : null;
  });

  sample(): void {
    this.a.set(SAMPLE.a);
    this.b.set(SAMPLE.b);
    this.aName.set(null);
    this.bName.set(null);
  }

  clear(): void {
    this.a.set('');
    this.b.set('');
    this.aName.set(null);
    this.bName.set(null);
  }

  protected ignores(option: Option): boolean {
    return this.ignored().has(option);
  }

  protected onOption(option: Option, event: Event): void {
    const next = new Set(this.ignored());
    if ((event.target as HTMLInputElement).checked) next.add(option);
    else next.delete(option);
    this.ignored.set(next);
  }

  protected onInput(side: 'a' | 'b', event: Event): void {
    const text = (event.target as HTMLTextAreaElement).value;
    (side === 'a' ? this.a : this.b).set(text);
    (side === 'a' ? this.aName : this.bName).set(null);
  }

  protected onGranularity(id: string): void {
    this.granularity.set(id as Granularity);
  }

  protected onLayout(id: string): void {
    this.layout.set(id as Layout);
  }

  private swap(): void {
    const [a, b, aName, bName] = [this.a(), this.b(), this.aName(), this.bName()];
    this.a.set(b);
    this.b.set(a);
    this.aName.set(bName);
    this.bName.set(aName);
  }

  private async copyUnified(): Promise<void> {
    const unified = this.unified();
    if (unified !== null) {
      await this.clipboard.copy(unified);
    }
  }
}
