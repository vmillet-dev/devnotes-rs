import { ToolsRepository } from '@core/data/tools.repository';
import {
  Base64Decoded,
  Base64FileAnswer,
  Base64Options,
  Base64Saved,
  CaseConversion,
  ColourAnswer,
  ColourRequest,
  ConvertAnswer,
  ConvertRequest,
  CronAnswer,
  CronRequest,
  GenerateAnswer,
  GenerateRequest,
  HashAnswer,
  HashRequest,
  InstantAnswer,
  InstantRequest,
  JsonDiffAnswer,
  JsonDiffRequest,
  LineBreaksAnswer,
  LineBreaksRequest,
  LoremAnswer,
  LoremRequest,
  PasswordAnswer,
  PasswordRequest,
  SizesAnswer,
  SizesRequest,
  SlugRequest,
  UrlAnswer,
  UrlCodecAnswer,
  UrlCodecRequest,
  UuidInspection,
  UuidRequest,
  ZoneEntry,
  ZonesAnswer,
  ZonesRequest,
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
  colour: ColourAnswer = { colour: { kind: 'empty' }, against: { kind: 'empty' }, contrast: null };
  conversion: ConvertAnswer = { kind: 'converted', text: '' };
  generated: GenerateAnswer = { kind: 'generated', text: '{}', seed: 1, unsupported: [] };
  loremAnswer: LoremAnswer = { text: '', seed: 1 };
  diffAnswer: JsonDiffAnswer = {
    kind: 'compared',
    changes: [],
    counts: { added: 0, removed: 0, modified: 0 },
    rows: [],
    rowsTruncated: false,
    patch: '[]',
  };
  instant: InstantAnswer = { kind: 'unreadable', at: 1 };
  now = '2026-09-29T12:03:12.000Z';
  zonesFound: ZoneEntry[] = [];
  zonesAnswer: ZonesAnswer = { kind: 'unreadable', at: 1 };
  wallClock = '2026-09-29 14:03:12';
  cron: CronAnswer = { kind: 'reboot' };
  sizes: SizesAnswer = { kind: 'tooLarge' };

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

  describeColour(request: ColourRequest): Promise<ColourAnswer> {
    return this.answer('describe_colour', request, this.colour);
  }

  convert(request: ConvertRequest): Promise<ConvertAnswer> {
    return this.answer('convert_data', request, this.conversion);
  }

  generateJson(request: GenerateRequest): Promise<GenerateAnswer> {
    return this.answer('generate_json', request, this.generated);
  }

  lorem(request: LoremRequest): Promise<LoremAnswer> {
    return this.answer('lorem_ipsum', request, this.loremAnswer);
  }

  diffJson(request: JsonDiffRequest): Promise<JsonDiffAnswer> {
    return this.answer('diff_json', request, this.diffAnswer);
  }

  describeInstant(request: InstantRequest): Promise<InstantAnswer> {
    return this.answer('describe_instant', request, this.instant);
  }

  currentInstant(): Promise<string> {
    return this.answer('current_instant', null, this.now);
  }

  searchTimeZones(query: string): Promise<ZoneEntry[]> {
    return this.answer('search_time_zones', query, this.zonesFound);
  }

  placeInZones(request: ZonesRequest): Promise<ZonesAnswer> {
    return this.answer('place_in_zones', request, this.zonesAnswer);
  }

  timeInZone(zone: string | null): Promise<string> {
    return this.answer('time_in_zone', zone, this.wallClock);
  }

  describeCron(request: CronRequest): Promise<CronAnswer> {
    return this.answer('describe_cron', request, this.cron);
  }

  convertSize(request: SizesRequest): Promise<SizesAnswer> {
    return this.answer('convert_size', request, this.sizes);
  }

  requestsOf(command: string): unknown[] {
    return this.asked.filter((entry) => entry.command === command).map((entry) => entry.request);
  }

  private answer<T>(command: string, request: unknown, value: T): Promise<T> {
    this.asked.push({ command, request });
    return Promise.resolve(value);
  }
}
