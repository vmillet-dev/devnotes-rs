import { FormatAnswer, FormatRequest } from '@core/services/format/format.model';
import { PrettierAdapter } from '@core/services/format/formatter.service';

/** Answers what the spec sets, and records what it was asked: jsdom has no `Worker`. */
export class FakePrettier implements PrettierAdapter {
  answer: FormatAnswer = { kind: 'unchanged' };
  readonly requests: FormatRequest[] = [];

  run(request: FormatRequest): Promise<FormatAnswer> {
    this.requests.push(request);
    return Promise.resolve(this.answer);
  }
}
