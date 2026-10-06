import { ToolsRepository } from '@core/data/tools.repository';
import {
  Base64FileAnswer,
  CaseAnswer,
  CaseRequest,
  CheckAnswer,
  CheckRequest,
  ColourAnswer,
  ColourRequest,
  ConvertAnswer,
  ConvertRequest,
  CronAnswer,
  CronRequest,
  DurationsAnswer,
  DurationsRequest,
  DecodeAnswer,
  DecodeRequest,
  EncodeRequest,
  Encodings,
  GenerateAnswer,
  GenerateRequest,
  HashAnswer,
  HashRequest,
  IdInspection,
  IdentifiersRequest,
  InstantAnswer,
  InstantRequest,
  JsonDiffAnswer,
  JwtAnswer,
  JwtRequest,
  JsonDiffRequest,
  LineBreaksAnswer,
  LineBreaksRequest,
  LoremAnswer,
  LoremRequest,
  PasswordAnswer,
  PercentagesAnswer,
  PercentagesRequest,
  PermissionsAnswer,
  PermissionsRequest,
  PasswordRequest,
  SavedBytes,
  SizesAnswer,
  SizesRequest,
  StatsRequest,
  TextDiffAnswer,
  TextDiffRequest,
  TextStats,
  TransferAnswer,
  TransferRequest,
  UrlAnswer,
  UrlCodecAnswer,
  UrlCodecRequest,
  ZoneEntry,
  ZonesAnswer,
  ZonesRequest,
} from '@core/model/tool-answers.model';

/** Answers what a spec sets and records what was asked: the tools' rules are Rust's. */
export class FakeToolsRepository implements Pick<ToolsRepository, keyof ToolsRepository> {
  readonly asked: { readonly command: string; readonly request: unknown }[] = [];

  caseAnswer: CaseAnswer = { conversions: [], words: [], slug: '' };
  lineBreaks: LineBreaksAnswer = {
    found: { lf: 0, crlf: 0, cr: 0 },
    text: '',
    converted: 0,
    trimmed: 0,
    finalNewline: 'unchanged',
  };
  url: UrlAnswer = { kind: 'invalid', problem: 'empty' };
  urlCodecAnswer: UrlCodecAnswer = { kind: 'done', text: '' };
  encodings: Encodings = {
    characters: 0,
    bytes: 0,
    wide: null,
    base64: '',
    base64Url: '',
    base32: '',
    hex: '',
    binary: '',
    decimal: '',
  };
  base64File: Base64FileAnswer = { kind: 'failed', problem: 'notFound' };
  decodedBytes: DecodeAnswer = {
    readAs: 'base64',
    guessed: true,
    also: [],
    decoded: { kind: 'text', text: '', bytes: 0 },
  };
  savedBytes: SavedBytes = { kind: 'saved', bytes: 0 };
  hashAnswer: HashAnswer = {
    kind: 'hashed',
    bytes: 0,
    endsWithNewline: false,
    digests: [],
    verdict: null,
  };
  passwords: PasswordAnswer = { kind: 'noCharacters' };
  identifiers: string[] = [];
  inspection: IdInspection = { kind: 'unrecognised' };
  durations: DurationsAnswer = { zone: 'Europe/Paris', gap: null, duration: null };
  colour: ColourAnswer = { colour: { kind: 'empty' }, against: { kind: 'empty' }, contrast: null };
  conversion: ConvertAnswer = { kind: 'converted', text: '', dropped: [] };
  generated: GenerateAnswer = { kind: 'generated', text: '{}', seed: 1, unsupported: [] };
  loremAnswer: LoremAnswer = { text: '', seed: 1, count: 3, atMost: 200 };
  diffAnswer: JsonDiffAnswer = {
    kind: 'compared',
    changes: [],
    counts: { added: 0, removed: 0, modified: 0 },
    rows: [],
    rowsTruncated: false,
    patch: '[]',
    formatA: 'json',
    formatB: 'json',
  };
  textDiff: TextDiffAnswer = {
    added: 0,
    removed: 0,
    rows: [],
    rowsTruncated: false,
    unified: '',
    identical: true,
    bothJson: false,
  };
  instant: InstantAnswer = { kind: 'unreadable', at: 1 };
  now = '2026-09-29T12:03:12.000Z';
  zonesFound: ZoneEntry[] = [];
  zonesAnswer: ZonesAnswer = { kind: 'unreadable', at: 1 };
  wallClock = '2026-09-29 14:03:12';
  cron: CronAnswer = { kind: 'reboot' };
  sizes: SizesAnswer = { kind: 'tooLarge' };
  transfer: TransferAnswer = { kind: 'zeroRate' };
  checks: CheckAnswer = { kind: 'tooShort' };
  stats: TextStats = {
    characters: 0,
    codePoints: 0,
    utf16Units: 0,
    utf8Bytes: 0,
    nonWhitespace: 0,
    words: 0,
    lines: 0,
    nonEmptyLines: 0,
    paragraphs: 0,
    distinct: 0,
    frequencies: [],
    frequenciesTruncated: false,
  };
  jwt: JwtAnswer = { kind: 'malformed', problem: 'segmentCount', segment: null, segments: 1, at: null };
  permissions: PermissionsAnswer = { mode: { kind: 'empty' }, umask: { kind: 'empty' } };
  percentages: PercentagesAnswer = {
    of: { kind: 'empty' },
    share: { kind: 'empty' },
    change: { kind: 'empty' },
    apply: { kind: 'empty' },
    before: { kind: 'empty' },
  };

  convertCase(request: CaseRequest): Promise<CaseAnswer> {
    return this.answer('convert_case', request, this.caseAnswer);
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

  encodeBytes(request: EncodeRequest): Promise<Encodings> {
    return this.answer('encode_bytes', request, this.encodings);
  }

  encodeBase64File(path: string): Promise<Base64FileAnswer> {
    return this.answer('encode_base64_file', path, this.base64File);
  }

  decodeBytes(request: DecodeRequest): Promise<DecodeAnswer> {
    return this.answer('decode_bytes', request, this.decodedBytes);
  }

  saveBytes(request: DecodeRequest, path: string): Promise<SavedBytes> {
    return this.answer('save_bytes', { request, path }, this.savedBytes);
  }

  hash(request: HashRequest): Promise<HashAnswer> {
    return this.answer('hash_input', request, this.hashAnswer);
  }

  generatePasswords(request: PasswordRequest): Promise<PasswordAnswer> {
    return this.answer('generate_passwords', request, this.passwords);
  }

  generateIdentifiers(request: IdentifiersRequest): Promise<string[]> {
    return this.answer('generate_identifiers', request, this.identifiers);
  }

  inspectIdentifier(text: string): Promise<IdInspection> {
    return this.answer('inspect_identifier', text, this.inspection);
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

  diffText(request: TextDiffRequest): Promise<TextDiffAnswer> {
    return this.answer('diff_text', request, this.textDiff);
  }

  describeInstant(request: InstantRequest): Promise<InstantAnswer> {
    return this.answer('describe_instant', request, this.instant);
  }

  measureDurations(request: DurationsRequest): Promise<DurationsAnswer> {
    return this.answer('measure_durations', request, this.durations);
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

  estimateTransfer(request: TransferRequest): Promise<TransferAnswer> {
    return this.answer('estimate_transfer', request, this.transfer);
  }

  convertSize(request: SizesRequest): Promise<SizesAnswer> {
    return this.answer('convert_size', request, this.sizes);
  }

  answerPercentages(request: PercentagesRequest): Promise<PercentagesAnswer> {
    return this.answer('answer_percentages', request, this.percentages);
  }

  describePermissions(request: PermissionsRequest): Promise<PermissionsAnswer> {
    return this.answer('describe_permissions', request, this.permissions);
  }

  checkDigits(request: CheckRequest): Promise<CheckAnswer> {
    return this.answer('check_digits', request, this.checks);
  }

  decodeJwt(request: JwtRequest): Promise<JwtAnswer> {
    return this.answer('decode_jwt', request, this.jwt);
  }

  textStats(request: StatsRequest): Promise<TextStats> {
    return this.answer('text_stats', request, this.stats);
  }

  requestsOf(command: string): unknown[] {
    return this.asked.filter((entry) => entry.command === command).map((entry) => entry.request);
  }

  private answer<T>(command: string, request: unknown, value: T): Promise<T> {
    this.asked.push({ command, request });
    return Promise.resolve(value);
  }
}
