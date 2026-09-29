import { ToolsRepository } from '@core/data/tools.repository';
import {
  Base64Decoded,
  Base64FileAnswer,
  Base64Options,
  Base64Saved,
  CaseConversion,
  HashAnswer,
  HashRequest,
  LineBreaksAnswer,
  LineBreaksRequest,
  PasswordAnswer,
  PasswordRequest,
  SlugRequest,
  UrlAnswer,
  UrlCodecAnswer,
  UrlCodecRequest,
  UuidInspection,
  UuidRequest,
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
  urlCodecAnswer: UrlCodecAnswer = { kind: 'done', text: '' };
  base64 = '';
  base64File: Base64FileAnswer = { kind: 'failed', problem: 'notFound' };
  base64Decoded: Base64Decoded = { kind: 'text', text: '', bytes: 0 };
  base64Saved: Base64Saved = { kind: 'saved', bytes: 0 };
  hashAnswer: HashAnswer = {
    kind: 'hashed',
    bytes: 0,
    endsWithNewline: false,
    digests: [],
    recognised: null,
  };
  passwords: PasswordAnswer = { kind: 'noCharacters' };
  uuids: string[] = [];
  uuidInspection: UuidInspection = { kind: 'invalid' };

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

  urlCodec(request: UrlCodecRequest): Promise<UrlCodecAnswer> {
    return this.answer('url_codec', request, this.urlCodecAnswer);
  }

  encodeBase64(text: string, options: Base64Options): Promise<string> {
    return this.answer('encode_base64', { text, options }, this.base64);
  }

  encodeBase64File(path: string, options: Base64Options): Promise<Base64FileAnswer> {
    return this.answer('encode_base64_file', { path, options }, this.base64File);
  }

  decodeBase64(text: string, options: Base64Options): Promise<Base64Decoded> {
    return this.answer('decode_base64', { text, options }, this.base64Decoded);
  }

  saveBase64(text: string, options: Base64Options, path: string): Promise<Base64Saved> {
    return this.answer('save_base64', { text, options, path }, this.base64Saved);
  }

  hash(request: HashRequest): Promise<HashAnswer> {
    return this.answer('hash_input', request, this.hashAnswer);
  }

  generatePasswords(request: PasswordRequest): Promise<PasswordAnswer> {
    return this.answer('generate_passwords', request, this.passwords);
  }

  generateUuids(request: UuidRequest): Promise<string[]> {
    return this.answer('generate_uuids', request, this.uuids);
  }

  inspectUuid(text: string): Promise<UuidInspection> {
    return this.answer('inspect_uuid', text, this.uuidInspection);
  }

  requestsOf(command: string): unknown[] {
    return this.asked.filter((entry) => entry.command === command).map((entry) => entry.request);
  }

  private answer<T>(command: string, request: unknown, value: T): Promise<T> {
    this.asked.push({ command, request });
    return Promise.resolve(value);
  }
}
