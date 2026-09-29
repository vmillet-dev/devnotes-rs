import { ToolsRepository } from '@core/data/tools.repository';
import {
  CaseConversion,
  LineBreaksAnswer,
  LineBreaksRequest,
  SlugRequest,
  UrlAnswer,
} from '@core/model/tool-answers.model';

/** Answers what a spec sets and records what was asked: the tools' rules are Rust's. */
export class FakeToolsRepository implements Pick<ToolsRepository, keyof ToolsRepository> {
  readonly asked: { readonly command: string; readonly request: unknown }[] = [];

  cases: CaseConversion[] = [];
  slug = '';
  lineBreaks: LineBreaksAnswer = {
    found: { lf: 0, crlf: 0, cr: 0 },
    text: '',
    converted: 0,
    trimmed: 0,
    finalNewline: 'unchanged',
  };
  url: UrlAnswer = { kind: 'invalid', problem: 'empty' };

  convertCase(text: string): Promise<CaseConversion[]> {
    return this.answer('convert_case', text, this.cases);
  }

  slugify(request: SlugRequest): Promise<string> {
    return this.answer('slugify', request, this.slug);
  }

  fixLineBreaks(request: LineBreaksRequest): Promise<LineBreaksAnswer> {
    return this.answer('fix_line_breaks', request, this.lineBreaks);
  }

  parseUrl(text: string): Promise<UrlAnswer> {
    return this.answer('parse_url', text, this.url);
  }

  requestsOf(command: string): unknown[] {
    return this.asked.filter((entry) => entry.command === command).map((entry) => entry.request);
  }

  private answer<T>(command: string, request: unknown, value: T): Promise<T> {
    this.asked.push({ command, request });
    return Promise.resolve(value);
  }
}
