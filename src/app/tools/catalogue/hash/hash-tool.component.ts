import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { ToolsRepository } from '@core/data/tools.repository';
import { DigestEncoding, DigestShape, HashAlgorithm, HashInput } from '@core/model/tool-answers.model';
import { FileDialogService } from '@core/services/dialogs/file-dialog.service';
import { liveResult } from '@core/services/tools/live-result';
import { Tool, ToolResult } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import {
  Segment,
  SegmentedChoiceComponent,
} from '@shared/controls/segmented-choice/segmented-choice.component';
import { IconComponent } from '@shared/icon/icon.component';
import { ResultRowComponent } from '@tools/ui/result-row/result-row.component';

type Source = 'text' | 'file';

const ALGORITHMS: readonly HashAlgorithm[] = ['md5', 'sha1', 'sha256', 'sha384', 'sha512', 'sha3-256'];

const NAMES: Record<HashAlgorithm, string> = {
  md5: 'MD5',
  sha1: 'SHA-1',
  sha256: 'SHA-256',
  sha384: 'SHA-384',
  sha512: 'SHA-512',
  'sha3-256': 'SHA3-256',
};

const HMAC_NAMES: Record<HashAlgorithm, string> = {
  md5: 'HMAC-MD5',
  sha1: 'HMAC-SHA1',
  sha256: 'HMAC-SHA256',
  sha384: 'HMAC-SHA384',
  sha512: 'HMAC-SHA512',
  'sha3-256': 'HMAC-SHA3-256',
};

const OBSOLETE: readonly HashAlgorithm[] = ['md5', 'sha1'];

/** "64 caractères hexadécimaux : SHA-256 ou SHA3-256", once translated. */
interface ShapeView {
  key: string;
  count: number;
  names: string[];
}

function shapeView(shape: DigestShape): ShapeView {
  switch (shape.kind) {
    case 'hex':
    case 'base64':
      return {
        key: `tools.hash.shapes.${shape.kind}${shape.algorithms.length === 0 ? 'None' : ''}`,
        count: shape.kind === 'hex' ? shape.characters : shape.bytes,
        names: shape.algorithms.map((algorithm) => NAMES[algorithm]),
      };
    case 'unknown':
      return { key: 'tools.hash.shapes.unknown', count: 0, names: [] };
  }
}

const SOURCES: readonly Segment[] = (['text', 'file'] as const).map((id) => ({
  id,
  labelKey: `tools.hash.sources.${id}`,
}));

const ENCODINGS: readonly Segment[] = (['hex', 'base64'] as const).map((id) => ({
  id,
  labelKey: `tools.hash.encodings.${id}`,
}));

/** SHA-256 of `hello world`, so the check finds its algorithm. */
const SAMPLE = {
  text: 'hello world',
  expected: 'b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9',
};

/**
 * ⚠️ The HMAC key is a plain signal, never a `toolState`: it dies with the tool, and nothing in
 * the result carries it. The switch alone is remembered.
 */
@Component({
  selector: 'app-hash-tool',
  imports: [IconComponent, ResultRowComponent, SegmentedChoiceComponent, TranslocoPipe],
  templateUrl: './hash-tool.component.html',
  styleUrl: './hash-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class HashToolComponent implements Tool {
  private readonly repository = inject(ToolsRepository);
  private readonly files = inject(FileDialogService);

  protected readonly source = toolState<Source>('hash.source', 'text');
  protected readonly text = toolState('hash.text', '');
  protected readonly path = toolState<string | null>('hash.path', null);
  protected readonly algorithms = toolState<readonly HashAlgorithm[]>('hash.algorithms', ALGORITHMS);
  protected readonly encoding = toolState<DigestEncoding>('hash.encoding', 'hex');
  protected readonly hmac = toolState('hash.hmac', false);
  protected readonly expected = toolState('hash.expected', '');

  protected readonly key = signal('');
  protected readonly keyShown = signal(false);

  protected readonly sources = SOURCES;
  protected readonly encodings = ENCODINGS;
  protected readonly allAlgorithms = ALGORITHMS;
  protected readonly names = NAMES;

  /** Keyed only once there is a key: a switch on over an empty field hashes plainly. */
  private readonly keyed = computed(() => this.hmac() && this.key() !== '');

  private readonly input = computed<HashInput | undefined>(() => {
    const path = this.path();
    if (this.source() === 'file') return path === null ? undefined : { kind: 'file', path };
    return this.text() === '' ? undefined : { kind: 'text', text: this.text() };
  });

  /** A pasted digest alone is still asked about: Rust reads its shape. */
  protected readonly answer = liveResult(
    () => {
      const input = this.input();
      const expected = this.expected().trim() === '' ? null : this.expected();
      return input === undefined && expected === null
        ? undefined
        : {
            input: input ?? null,
            algorithms: [...this.algorithms()],
            encoding: this.encoding(),
            key: input !== undefined && this.keyed() ? this.key() : null,
            expected,
          };
    },
    (request) => this.repository.hash(request),
  );

  private readonly digestNames = computed(() =>
    (this.answer.answered()?.key ?? null) === null ? NAMES : HMAC_NAMES,
  );

  protected readonly hashed = computed(() => {
    const answer = this.answer.value();
    return answer?.kind === 'hashed' ? answer : null;
  });

  protected readonly problem = computed(() => {
    const answer = this.answer.value();
    return answer?.kind === 'failed' ? answer.problem : null;
  });

  protected readonly digests = computed(() => {
    const hashed = this.hashed();
    const names = this.digestNames();
    const verdict = hashed?.verdict;
    return (hashed?.digests ?? []).map((digest) => ({
      ...digest,
      name: names[digest.algorithm],
      obsolete: OBSOLETE.includes(digest.algorithm),
      matched: verdict?.kind === 'matches' && verdict.algorithm === digest.algorithm,
    }));
  });

  /** "HMAC-SHA256, en hex" when it matches; otherwise what the pasted digest looks like. */
  protected readonly verdict = computed(() => {
    const answer = this.answer.value();
    if (answer?.kind === 'shaped') return { kind: 'shaped' as const, shape: shapeView(answer.shape) };
    const verdict = answer?.kind === 'hashed' ? answer.verdict : null;
    if (!verdict) return null;
    return verdict.kind === 'matches'
      ? { kind: 'matches' as const, name: this.digestNames()[verdict.algorithm], encoding: verdict.encoding }
      : { kind: 'noMatch' as const, shape: shapeView(verdict.shape) };
  });

  protected readonly matched = computed(() => {
    const verdict = this.verdict();
    return verdict?.kind === 'matches' ? verdict : null;
  });

  protected readonly shape = computed(() => {
    const verdict = this.verdict();
    return verdict && verdict.kind !== 'matches' ? verdict.shape : null;
  });

  protected readonly weakChosen = computed(() => this.algorithms().some((id) => OBSOLETE.includes(id)));

  readonly result = computed<ToolResult | null>(() => {
    const digests = this.digests();
    if (digests.length === 0) return null;

    const width = Math.max(...digests.map((digest) => digest.name.length));
    return {
      title: { key: 'tools.hash.noteTitle' },
      kind: 'snippet',
      language: 'txt',
      content: digests.map((digest) => `${digest.name.padEnd(width)}  ${digest.value}`).join('\n'),
    };
  });

  sample(): void {
    this.source.set('text');
    this.text.set(SAMPLE.text);
    this.expected.set(SAMPLE.expected);
  }

  clear(): void {
    this.text.set('');
    this.path.set(null);
    this.expected.set('');
    this.key.set('');
  }

  protected isChosen(algorithm: HashAlgorithm): boolean {
    return this.algorithms().includes(algorithm);
  }

  protected toggle(algorithm: HashAlgorithm): void {
    const chosen = this.algorithms();
    this.algorithms.set(
      chosen.includes(algorithm)
        ? chosen.filter((id) => id !== algorithm)
        : ALGORITHMS.filter((id) => id === algorithm || chosen.includes(id)),
    );
  }

  protected onText(event: Event): void {
    this.text.set((event.target as HTMLTextAreaElement).value);
  }

  protected onKey(event: Event): void {
    this.key.set((event.target as HTMLInputElement).value);
  }

  protected onExpected(event: Event): void {
    this.expected.set((event.target as HTMLInputElement).value);
  }

  protected onHmac(event: Event): void {
    this.hmac.set((event.target as HTMLInputElement).checked);
  }

  protected onSource(id: string): void {
    this.source.set(id as Source);
  }

  protected onEncoding(id: string): void {
    this.encoding.set(id as DigestEncoding);
  }

  protected async pickFile(): Promise<void> {
    const path = await this.files.pickFile();
    if (path !== null) {
      this.path.set(path);
    }
  }

  protected fileName(path: string): string {
    return path.split(/[\\/]/).at(-1) ?? path;
  }
}
