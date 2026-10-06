import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { ToolsRepository } from '@core/data/tools.repository';
import { ByteEncoding, Encodings } from '@core/model/tool-answers.model';
import { FileDialogService } from '@core/services/dialogs/file-dialog.service';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { StatusNotifier } from '@core/services/notifications/status.service';
import { liveResult } from '@core/services/tools/live-result';
import { Tool, ToolResult } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import {
  Segment,
  SegmentedChoiceComponent,
} from '@shared/controls/segmented-choice/segmented-choice.component';
import { CopyValueComponent } from '@tools/ui/copy-value/copy-value.component';
import { ResultRowComponent } from '@tools/ui/result-row/result-row.component';

type Direction = 'encode' | 'decode';
type Source = 'text' | 'file';

const AUTO = 'auto';

const segments = <T extends string>(ids: readonly T[], prefix: string): readonly Segment[] =>
  ids.map((id) => ({ id, labelKey: `${prefix}.${id}` }));

/** The six rows, in the order they are drawn. */
const ROWS = [
  'base64',
  'base64Url',
  'base32',
  'hex',
  'binary',
  'decimal',
] as const satisfies readonly (keyof Encodings)[];

const READINGS: readonly ByteEncoding[] = ['base64', 'base64Url', 'base32', 'hex', 'binary', 'decimal'];

/** What the page draws of a long result; the copy button still copies it whole. */
const SHOWN_CHARACTERS = 4000;

const shown = (text: string): string =>
  text.length > SHOWN_CHARACTERS ? `${text.slice(0, SHOWN_CHARACTERS)}…` : text;

/** A text in six encodings at once, and back from whichever one Rust recognises. */
@Component({
  selector: 'app-base64-tool',
  imports: [CopyValueComponent, ResultRowComponent, SegmentedChoiceComponent, TranslocoPipe],
  templateUrl: './base64-tool.component.html',
  styleUrl: './base64-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class Base64ToolComponent implements Tool {
  private readonly repository = inject(ToolsRepository);
  private readonly files = inject(FileDialogService);
  private readonly notifier = inject(ErrorNotifier);
  private readonly status = inject(StatusNotifier);

  protected readonly direction = toolState<Direction>('base64.direction', 'encode');
  protected readonly source = toolState<Source>('base64.source', 'text');
  protected readonly text = toolState('base64.text', '');
  protected readonly path = toolState<string | null>('base64.path', null);
  protected readonly spaced = toolState('base64.spaced', true);
  protected readonly uppercase = toolState('base64.uppercase', false);
  protected readonly reading = toolState<ByteEncoding | null>('base64.reading', null);

  protected readonly directions = segments(['encode', 'decode'] as const, 'tools.base64.directions');
  protected readonly sources = segments(['text', 'file'] as const, 'tools.base64.sources');
  protected readonly readings = segments([AUTO, ...READINGS], 'tools.base64.readings');

  protected readonly encodedText = liveResult(
    () =>
      this.direction() === 'encode' && this.source() === 'text' && this.text() !== ''
        ? { text: this.text(), spaced: this.spaced(), uppercase: this.uppercase() }
        : undefined,
    (request) => this.repository.encodeBytes(request),
  );

  protected readonly encodedFile = liveResult(
    () => {
      const path = this.path();
      return this.direction() === 'encode' && this.source() === 'file' && path !== null ? path : undefined;
    },
    (path) => this.repository.encodeBase64File(path),
  );

  protected readonly decoded = liveResult(
    () =>
      this.direction() === 'decode' && this.text().trim() !== ''
        ? { text: this.text(), reading: this.reading() }
        : undefined,
    (request) => this.repository.decodeBytes(request),
  );

  /** The rows of a text's encodings, or a file's two Base64. */
  protected readonly rows = computed(() => {
    if (this.source() === 'file') {
      const file = this.encodedFile.value();
      return file?.kind === 'encoded'
        ? [
            { id: 'base64', value: file.base64 },
            { id: 'base64Url', value: file.base64Url },
          ].map((row) => ({ ...row, shown: shown(row.value) }))
        : [];
    }
    const encodings = this.encodedText.value();
    return encodings ? ROWS.map((id) => ({ id, value: encodings[id], shown: shown(encodings[id]) })) : [];
  });

  protected readonly facts = computed(() => (this.source() === 'text' ? this.encodedText.value() : null));

  protected readonly decodedText = computed(() => {
    const decoded = this.decoded.value()?.decoded;
    return decoded?.kind === 'text' ? decoded.text : null;
  });

  protected readonly decodedBinary = computed(() => {
    const decoded = this.decoded.value()?.decoded;
    return decoded?.kind === 'binary' ? decoded : null;
  });

  protected readonly decodingProblem = computed(() => {
    const decoded = this.decoded.value()?.decoded;
    return decoded?.kind === 'invalid' ? decoded : null;
  });

  readonly result = computed<ToolResult | null>(() => {
    if (this.direction() === 'decode') {
      const text = this.decodedText();
      return text
        ? { title: { key: 'tools.base64.noteDecoded' }, kind: 'snippet', language: 'txt', content: text }
        : null;
    }
    const rows = this.rows();
    if (rows.length === 0) return null;
    const width = Math.max(...rows.map((row) => row.id.length));
    return {
      title: { key: 'tools.base64.noteEncoded' },
      kind: 'snippet',
      language: 'txt',
      content: rows.map((row) => `${row.id.padEnd(width)}  ${row.value}`).join('\n'),
    };
  });

  sample(): void {
    this.direction.set('encode');
    this.source.set('text');
    this.text.set('Café');
  }

  clear(): void {
    this.text.set('');
    this.path.set(null);
  }

  protected onInput(event: Event): void {
    this.text.set((event.target as HTMLTextAreaElement).value);
  }

  protected onSpaced(event: Event): void {
    this.spaced.set((event.target as HTMLInputElement).checked);
  }

  protected onUppercase(event: Event): void {
    this.uppercase.set((event.target as HTMLInputElement).checked);
  }

  protected choose(state: 'direction' | 'source' | 'reading', id: string): void {
    if (state === 'direction') this.direction.set(id as Direction);
    if (state === 'source') this.source.set(id as Source);
    if (state === 'reading') this.reading.set(id === AUTO ? null : (id as ByteEncoding));
  }

  protected async pickFile(): Promise<void> {
    const path = await this.files.pickFile();
    if (path !== null) {
      this.path.set(path);
    }
  }

  /** Bytes that are no text cannot be a note, nor shown: they are written where the user says. */
  protected async saveAsFile(): Promise<void> {
    const path = await this.files.chooseDestination('decoded.bin');
    if (path === null) return;

    const saved = await this.notifier.attempt('errors.toolFailed', () =>
      this.repository.saveBytes({ text: this.text(), reading: this.reading() }, path),
    );
    if (saved?.kind === 'saved') {
      this.status.notify({ key: 'tools.base64.saved', params: { count: saved.bytes } });
    } else if (saved?.kind === 'failed') {
      this.notifier.notify({ ref: { key: `tools.files.${saved.problem}` } });
    }
  }
}
