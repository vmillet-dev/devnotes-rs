import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { ToolsRepository } from '@core/data/tools.repository';
import { UrlParts } from '@core/model/tool-answers.model';
import { liveResult } from '@core/services/tools/live-result';
import { Tool, ToolResult } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import { CopyValueComponent } from '@tools/ui/copy-value/copy-value.component';

interface PartRow {
  readonly id: string;
  readonly value: string;
}

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

@Component({
  selector: 'app-url-parser-tool',
  imports: [CopyValueComponent, TranslocoPipe],
  templateUrl: './url-parser-tool.component.html',
  styleUrl: './url-parser-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class UrlParserToolComponent implements Tool {
  private readonly repository = inject(ToolsRepository);

  protected readonly text = toolState('url-parser.text', '');

  protected readonly answer = liveResult(
    () => (this.text().trim() === '' ? undefined : this.text()),
    (text) => this.repository.parseUrl(text),
  );

  protected readonly parts = computed(() => {
    const answer = this.answer.value();
    return answer?.kind === 'parsed' ? answer.parts : null;
  });

  protected readonly rows = computed(() => {
    const parts = this.parts();
    return parts ? rows(parts) : [];
  });

  protected readonly problem = computed(() => {
    const answer = this.answer.value();
    return answer?.kind === 'invalid' ? answer.problem : null;
  });

  /** As JSON, so the note reads the same in any language; the password stays behind. */
  readonly result = computed<ToolResult | null>(() => {
    const parts = this.parts();
    if (parts === null) return null;

    const { password: _password, ...kept } = parts;
    return {
      title: { key: 'tools.url-parser.noteTitle', params: { host: parts.host ?? parts.scheme } },
      kind: 'snippet',
      language: 'json',
      content: JSON.stringify(kept, null, 2),
    };
  });

  sample(): void {
    this.text.set(SAMPLE);
  }

  clear(): void {
    this.text.set('');
  }

  protected onInput(event: Event): void {
    this.text.set((event.target as HTMLTextAreaElement).value);
  }
}
