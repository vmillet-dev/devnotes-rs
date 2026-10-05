import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { ToolsRepository } from '@core/data/tools.repository';
import { UrlParts, UrlScope } from '@core/model/tool-answers.model';
import { liveResult } from '@core/services/tools/live-result';
import { Tool, ToolResult } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import {
  Segment,
  SegmentedChoiceComponent,
} from '@shared/controls/segmented-choice/segmented-choice.component';
import { CopyValueComponent } from '@tools/ui/copy-value/copy-value.component';

type Side = 'decoded' | 'encoded';
type Tab = 'codec' | 'parse';

interface PartRow {
  readonly id: string;
  readonly value: string;
}

const TABS: readonly Segment[] = (['codec', 'parse'] as const).map((id) => ({
  id,
  labelKey: `tools.url-codec.tabs.${id}`,
}));

const SCOPES: readonly Segment[] = (['component', 'whole'] as const).map((id) => ({
  id,
  labelKey: `tools.url-codec.scopes.${id}`,
}));

/** Its query is encoded: decoded on one tab, its parameters read on the other. */
const SAMPLE =
  'https://ada:secret@api.example.com:8443/v2/search?q=caf%C3%A9%20cr%C3%A8me&tag=a&tag=b#results';

/** Written as the URL gives them: a password is never read back, only said to be there. */
function rows(parts: UrlParts): PartRow[] {
  const port = parts.port === null ? null : String(parts.port);
  return [
    { id: 'scheme', value: parts.scheme },
    { id: 'username', value: parts.username },
    { id: 'password', value: parts.password === null ? null : '••••••' },
    { id: 'host', value: parts.host },
    { id: 'port', value: port },
    { id: 'path', value: parts.path },
    { id: 'query', value: parts.query },
    { id: 'fragment', value: parts.fragment },
  ].filter((row): row is PartRow => row.value !== null && row.value !== '');
}

/**
 * One text, two tabs. Encoding: both fields are typed into, the one typed in last is the text and
 * the other its translation. Analysing reads that same text.
 */
@Component({
  selector: 'app-url-codec-tool',
  imports: [CopyValueComponent, SegmentedChoiceComponent, TranslocoPipe],
  templateUrl: './url-codec-tool.component.html',
  styleUrl: './url-codec-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class UrlCodecToolComponent implements Tool {
  private readonly repository = inject(ToolsRepository);

  protected readonly tab = toolState<Tab>('url-codec.tab', 'codec');
  protected readonly text = toolState('url-codec.text', '');
  protected readonly side = toolState<Side>('url-codec.side', 'decoded');
  protected readonly scope = toolState<UrlScope>('url-codec.scope', 'component');

  protected readonly tabs = TABS;
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

  /** On the codec's tab, the decoded side is read too: a URL there is offered to the parser. */
  protected readonly parsed = liveResult(
    () => {
      const text = this.tab() === 'parse' ? this.text() : this.decoded();
      return text.trim() === '' ? undefined : text;
    },
    (text) => this.repository.parseUrl(text),
  );

  protected readonly parts = computed(() => {
    const answer = this.parsed.value();
    return answer?.kind === 'parsed' ? answer.parts : null;
  });

  protected readonly rows = computed(() => {
    const parts = this.parts();
    return parts ? rows(parts) : [];
  });

  protected readonly parseProblem = computed(() => {
    const answer = this.parsed.value();
    return answer?.kind === 'invalid' ? answer.problem : null;
  });

  protected readonly offersAnalysis = computed(
    () => this.tab() === 'codec' && this.parts() !== null && this.parsed.answered() === this.decoded(),
  );

  readonly result = computed<ToolResult | null>(() =>
    this.tab() === 'codec' ? this.codecResult() : this.partsResult(),
  );

  sample(): void {
    this.side.set('encoded');
    this.text.set(SAMPLE);
  }

  clear(): void {
    this.text.set('');
  }

  protected onTab(id: string): void {
    this.tab.set(id as Tab);
  }

  protected onInput(side: Side, event: Event): void {
    this.side.set(side);
    this.text.set((event.target as HTMLTextAreaElement).value);
  }

  protected onParsed(event: Event): void {
    this.text.set((event.target as HTMLTextAreaElement).value);
  }

  protected onScope(id: string): void {
    this.scope.set(id as UrlScope);
  }

  /** « Analyser cette URL »: the decoded side becomes the text, read on the other tab. */
  protected analyse(): void {
    this.text.set(this.decoded());
    this.side.set('decoded');
    this.tab.set('parse');
  }

  private codecResult(): ToolResult | null {
    const translated = this.translated();
    if (translated === '') return null;

    const encoding = this.answer.answered()?.direction === 'encode';
    return {
      title: { key: encoding ? 'tools.url-codec.noteEncoded' : 'tools.url-codec.noteDecoded' },
      kind: 'snippet',
      language: 'txt',
      content: translated,
    };
  }

  /** As JSON, so the note reads the same in any language; the password stays behind. */
  private partsResult(): ToolResult | null {
    const parts = this.parts();
    if (parts === null) return null;

    const { password: _password, ...kept } = parts;
    return {
      title: { key: 'tools.url-codec.noteParsed', params: { host: parts.host ?? parts.scheme } },
      kind: 'snippet',
      language: 'json',
      content: JSON.stringify(kept, null, 2),
    };
  }
}
