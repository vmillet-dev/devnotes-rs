import { Injectable } from '@angular/core';
import { commands } from '@core/ipc/bindings';
import { unwrap } from '@core/ipc/ipc.error';
import {
  Base64Decoded,
  Base64FileAnswer,
  Base64Options,
  Base64Saved,
  CaseConversion,
  CheckAnswer,
  CheckRequest,
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

  async urlCodec(request: UrlCodecRequest): Promise<UrlCodecAnswer> {
    return unwrap('url_codec', await commands.urlCodec(request));
  }

  async encodeBase64(text: string, options: Base64Options): Promise<string> {
    return unwrap('encode_base64', await commands.encodeBase64(text, options));
  }

  /** By its path: Rust reads the file, and its bytes never cross. */
  async encodeBase64File(path: string, options: Base64Options): Promise<Base64FileAnswer> {
    return unwrap('encode_base64_file', await commands.encodeBase64File(path, options));
  }

  async decodeBase64(text: string, options: Base64Options): Promise<Base64Decoded> {
    return unwrap('decode_base64', await commands.decodeBase64(text, options));
  }

  /** A file by its path, read in Rust a block at a time; the HMAC key is zeroed there. */
  async hash(request: HashRequest): Promise<HashAnswer> {
    return unwrap('hash_input', await commands.hashInput(request));
  }

  async saveBase64(text: string, options: Base64Options, path: string): Promise<Base64Saved> {
    return unwrap('save_base64', await commands.saveBase64(text, options, path));
  }

  async generatePasswords(request: PasswordRequest): Promise<PasswordAnswer> {
    return unwrap('generate_passwords', await commands.generatePasswords(request));
  }

  async generateUuids(request: UuidRequest): Promise<string[]> {
    return unwrap('generate_uuids', await commands.generateUuids(request));
  }

  async inspectUuid(text: string): Promise<UuidInspection> {
    return unwrap('inspect_uuid', await commands.inspectUuid(text));
  }

  async describeColour(request: ColourRequest): Promise<ColourAnswer> {
    return unwrap('describe_colour', await commands.describeColour(request));
  }

  async convert(request: ConvertRequest): Promise<ConvertAnswer> {
    return unwrap('convert_data', await commands.convertData(request));
  }

  async generateJson(request: GenerateRequest): Promise<GenerateAnswer> {
    return unwrap('generate_json', await commands.generateJson(request));
  }

  async lorem(request: LoremRequest): Promise<LoremAnswer> {
    return unwrap('lorem_ipsum', await commands.loremIpsum(request));
  }

  async diffJson(request: JsonDiffRequest): Promise<JsonDiffAnswer> {
    return unwrap('diff_json', await commands.diffJson(request));
  }

  /** Read in the machine's zone when written without an offset. */
  async describeInstant(request: InstantRequest): Promise<InstantAnswer> {
    return unwrap('describe_instant', await commands.describeInstant(request));
  }

  /** ISO 8601 in UTC, to the millisecond. */
  async currentInstant(): Promise<string> {
    return unwrap('current_instant', await commands.currentInstant());
  }

  async searchTimeZones(query: string): Promise<ZoneEntry[]> {
    return unwrap('search_time_zones', await commands.searchTimeZones(query));
  }

  /** `from` null is the machine's zone, which is always listed first. */
  async placeInZones(request: ZonesRequest): Promise<ZonesAnswer> {
    return unwrap('place_in_zones', await commands.placeInZones(request));
  }

  /** Read in the machine's zone unless another is named; the next runs are counted from now. */
  async describeCron(request: CronRequest): Promise<CronAnswer> {
    return unwrap('describe_cron', await commands.describeCron(request));
  }

  /** Exact: the rounded values are for reading, the exact ones for copying. */
  async convertSize(request: SizesRequest): Promise<SizesAnswer> {
    return unwrap('convert_size', await commands.convertSize(request));
  }

  /** The five questions at once, each on its own two numbers. */
  async answerPercentages(request: PercentagesRequest): Promise<PercentagesAnswer> {
    return unwrap('answer_percentages', await commands.answerPercentages(request));
  }

  async describePermissions(request: PermissionsRequest): Promise<PermissionsAnswer> {
    return unwrap('describe_permissions', await commands.describePermissions(request));
  }

  /** A card number crosses and is forgotten: nothing here keeps it. */
  async checkDigits(request: CheckRequest): Promise<CheckAnswer> {
    return unwrap('check_digits', await commands.checkDigits(request));
  }

  /** The token and the secret are zeroed in Rust once the answer is made. */
  async decodeJwt(request: JwtRequest): Promise<JwtAnswer> {
    return unwrap('decode_jwt', await commands.decodeJwt(request));
  }

  /** The wall clock of a zone now, as the field takes it. */
  async timeInZone(zone: string | null): Promise<string> {
    return unwrap('time_in_zone', await commands.timeInZone(zone));
  }
}
