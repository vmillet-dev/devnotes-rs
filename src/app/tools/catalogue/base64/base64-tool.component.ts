import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { ToolsRepository } from '@core/data/tools.repository';
import { Base64Alphabet, Base64Options } from '@core/model/tool-answers.model';
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

type Direction = 'encode' | 'decode';
type Source = 'text' | 'file';

const segments = <T extends string>(ids: readonly T[], prefix: string): readonly Segment[] =>
  ids.map((id) => ({ id, labelKey: `${prefix}.${id}` }));

/** What the page draws of a long result; the copy button still copies it whole. */
const SHOWN_CHARACTERS = 4000;

@Component({
  selector: 'app-base64-tool',
  imports: [CopyValueComponent, SegmentedChoiceComponent, TranslocoPipe],
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
  protected readonly alphabet = toolState<Base64Alphabet>('base64.alphabet', 'standard');
  protected readonly padded = toolState('base64.padded', true);

  protected readonly directions = segments(['encode', 'decode'] as const, 'tools.base64.directions');
  protected readonly sources = segments(['text', 'file'] as const, 'tools.base64.sources');
  protected readonly alphabets = segments(['standard', 'urlSafe'] as const, 'tools.base64.alphabets');

  private readonly options = computed<Base64Options>(() => ({
    alphabet: this.alphabet(),
    padded: this.padded(),
  }));

  private readonly encodedText = liveResult(
    () =>
      this.direction() === 'encode' && this.source() === 'text' && this.text() !== ''
        ? { text: this.text(), options: this.options() }
        : undefined,
    ({ text, options }) => this.repository.encodeBase64(text, options),
  );

  protected readonly encodedFile = liveResult(
    () => {
      const path = this.path();
      return this.direction() === 'encode' && this.source() === 'file' && path !== null
        ? { path, options: this.options() }
        : undefined;
    },
    ({ path, options }) => this.repository.encodeBase64File(path, options),
  );

  protected readonly decoded = liveResult(
    () =>
      this.direction() === 'decode' && this.text().trim() !== ''
        ? { text: this.text(), options: this.options() }
        : undefined,
    ({ text, options }) => this.repository.decodeBase64(text, options),
  );

  /** What the encoding gave, from a text or from a file. */
  protected readonly encoded = computed(() => {
    const file = this.encodedFile.value();
    return this.source() === 'file'
      ? file?.kind === 'encoded'
        ? file.text
        : null
      : this.encodedText.value();
  });

  protected readonly shown = computed(() => {
    const encoded = this.encoded() ?? '';
    return encoded.length > SHOWN_CHARACTERS ? `${encoded.slice(0, SHOWN_CHARACTERS)}…` : encoded;
  });

  protected readonly decodedText = computed(() => {
    const decoded = this.decoded.value();
    return decoded?.kind === 'text' ? decoded.text : null;
  });

  protected readonly decodedBinary = computed(() => {
    const decoded = this.decoded.value();
    return decoded?.kind === 'binary' ? decoded : null;
  });

  protected readonly decodingProblem = computed(() => {
    const decoded = this.decoded.value();
    return decoded?.kind === 'invalid' ? decoded : null;
  });

  readonly result = computed<ToolResult | null>(() => {
    const content = this.direction() === 'encode' ? this.encoded() : this.decodedText();
    if (!content) return null;

    return {
      title: { key: this.direction() === 'encode' ? 'tools.base64.noteEncoded' : 'tools.base64.noteDecoded' },
      kind: 'snippet',
      language: 'txt',
      content,
    };
  });

  clear(): void {
    this.text.set('');
    this.path.set(null);
  }

  protected onInput(event: Event): void {
    this.text.set((event.target as HTMLTextAreaElement).value);
  }

  protected onPadded(event: Event): void {
    this.padded.set((event.target as HTMLInputElement).checked);
  }

  protected choose(state: 'direction' | 'source' | 'alphabet', id: string): void {
    if (state === 'direction') this.direction.set(id as Direction);
    if (state === 'source') this.source.set(id as Source);
    if (state === 'alphabet') this.alphabet.set(id as Base64Alphabet);
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
      this.repository.saveBase64(this.text(), this.options(), path),
    );
    if (saved?.kind === 'saved') {
      this.status.notify({ key: 'tools.base64.saved', params: { count: saved.bytes } });
    } else if (saved?.kind === 'failed') {
      this.notifier.notify({ ref: { key: `tools.files.${saved.problem}` } });
    }
  }
}
