import { Injectable } from '@angular/core';
import { commands } from '@core/ipc/bindings';
import { unwrap } from '@core/ipc/ipc.error';
import {
  CaseConversion,
  LineBreaksAnswer,
  LineBreaksRequest,
  SlugRequest,
  UrlAnswer,
} from '@core/model/tool-answers.model';

/** One method per tool: a request in, an answer out, and nothing of the library read. */
@Injectable({ providedIn: 'root' })
export class ToolsRepository {
  async convertCase(text: string): Promise<CaseConversion[]> {
    return unwrap('convert_case', await commands.convertCase(text));
  }

  async slugify(request: SlugRequest): Promise<string> {
    return unwrap('slugify', await commands.slugify(request));
  }

  async fixLineBreaks(request: LineBreaksRequest): Promise<LineBreaksAnswer> {
    return unwrap('fix_line_breaks', await commands.fixLineBreaks(request));
  }

  async parseUrl(text: string): Promise<UrlAnswer> {
    return unwrap('parse_url', await commands.parseUrl(text));
  }
}
