import { Injectable } from '@angular/core';
import { commands } from '@core/ipc/bindings';
import { unwrap } from '@core/ipc/ipc.error';
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
  GenerateAnswer,
  GenerateRequest,
  HashAnswer,
  HashRequest,
  LineBreaksAnswer,
  LineBreaksRequest,
  LoremAnswer,
  LoremRequest,
  PasswordAnswer,
  PasswordRequest,
  SlugRequest,
  UrlAnswer,
  UrlCodecAnswer,
  UrlCodecRequest,
  UuidInspection,
  UuidRequest,
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
}
