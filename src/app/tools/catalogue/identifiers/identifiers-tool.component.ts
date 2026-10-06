import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { ToolsRepository } from '@core/data/tools.repository';
import { IdInspection, IdKind, NanoAlphabet } from '@core/model/tool-answers.model';
import { liveResult } from '@core/services/tools/live-result';
import { Tool, ToolResult } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import {
  Segment,
  SegmentedChoiceComponent,
} from '@shared/controls/segmented-choice/segmented-choice.component';
import { CopyValueComponent } from '@tools/ui/copy-value/copy-value.component';

/** Proper names, spelled alike in every language: a note keeps them as they are. */
const KIND_NAMES: Record<IdKind, string> = {
  uuidV4: 'UUID v4',
  uuidV7: 'UUID v7',
  ulid: 'ULID',
  nanoId: 'NanoID',
  cuid2: 'CUID2',
  objectId: 'ObjectId',
  ksuid: 'KSUID',
};

const KINDS: readonly Segment[] = (Object.keys(KIND_NAMES) as IdKind[]).map((id) => ({
  id,
  labelKey: `tools.identifiers.kinds.${id}`,
}));

/** The kinds whose letters carry no meaning in their case: hexadecimal and Crockford's base 32. */
const CASELESS: ReadonlySet<IdKind> = new Set(['uuidV4', 'uuidV7', 'ulid', 'objectId']);

const ALPHABETS: readonly Segment[] = (['urlSafe', 'alphanumeric', 'hexLower', 'digits'] as const).map(
  (id) => ({
    id,
    labelKey: `tools.identifiers.alphabets.${id}`,
  }),
);

const COUNT_BOUNDS = { min: 1, max: 500 } as const;

/** The ULID of its own specification: the instant it carries is read back. */
const SAMPLE_ID = '01ARZ3NDEKTSV4RRFFQ69G5FAV';

interface Part {
  text: string;
  kind: 'plain' | 'hyphen' | 'version';
}

/**
 * A UUID's hyphens dimmed and its version digit picked out. Presentation only: the 8-4-4-4-12
 * layout is fixed, so the digit is the 13th hex digit with or without the hyphens.
 */
function uuidParts(id: string): Part[] {
  const version = id.includes('-') ? 14 : 12;
  return [...id].reduce<Part[]>((parts, character, at) => {
    const kind: Part['kind'] = character === '-' ? 'hyphen' : at === version ? 'version' : 'plain';
    const last = parts.at(-1);
    if (last?.kind === kind && kind === 'plain') last.text += character;
    else parts.push({ text: character, kind });
    return parts;
  }, []);
}

/** "2024-09-25 23:05:01.488 UTC": the instant as Rust wrote it, read in any language. */
function readable(iso: string | null): string | null {
  return iso ? iso.replace('T', ' ').replace('Z', ' UTC') : null;
}

/** What a field takes, so a typed `r` is never read as « Générer à nouveau ». */
function isTyping(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLElement &&
    (target.isContentEditable || target.closest('input, textarea, select, [role="dialog"]') !== null)
  );
}

@Component({
  selector: 'app-identifiers-tool',
  imports: [CopyValueComponent, SegmentedChoiceComponent, TranslocoPipe],
  templateUrl: './identifiers-tool.component.html',
  styleUrl: './identifiers-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { '(document:keydown)': 'onKeydown($event)' },
})
export class IdentifiersToolComponent implements Tool {
  private readonly repository = inject(ToolsRepository);

  protected readonly kind = toolState<IdKind>('identifiers.kind', 'uuidV4');
  protected readonly count = toolState('identifiers.count', 5);
  protected readonly uppercase = toolState('identifiers.uppercase', false);
  protected readonly hyphens = toolState('identifiers.hyphens', true);
  protected readonly nanoLength = toolState('identifiers.nanoLength', 21);
  protected readonly nanoAlphabet = toolState<NanoAlphabet>('identifiers.nanoAlphabet', 'urlSafe');
  protected readonly checked = toolState('identifiers.checked', '');

  private readonly draw = signal(0);
  /** The row whose « Copié » shows, tinted while it does. */
  protected readonly copiedRow = signal<number | null>(null);

  protected readonly kinds = KINDS;
  protected readonly alphabets = ALPHABETS;
  protected readonly bounds = COUNT_BOUNDS;

  protected readonly caseless = computed(() => CASELESS.has(this.kind()));
  protected readonly isUuid = computed(() => this.kind() === 'uuidV4' || this.kind() === 'uuidV7');

  protected readonly generated = liveResult(
    () => ({
      kind: this.kind(),
      count: this.count(),
      uppercase: this.uppercase(),
      hyphens: this.hyphens(),
      nanoLength: this.nanoLength(),
      nanoAlphabet: this.nanoAlphabet(),
      draw: this.draw(),
    }),
    ({ draw: _draw, ...request }) => this.repository.generateIdentifiers(request),
  );

  protected readonly list = computed(() => (this.generated.value() ?? []).join('\n'));

  /** Each drawn identifier, cut for display by the kind it was drawn as. */
  protected readonly ids = computed(() => {
    const asked = this.generated.answered();
    const uuid = asked?.kind === 'uuidV4' || asked?.kind === 'uuidV7';
    return (this.generated.value() ?? []).map((text) => ({
      text,
      parts: uuid ? uuidParts(text) : [{ text, kind: 'plain' as const }],
    }));
  });

  protected readonly inspection = liveResult(
    () => (this.checked().trim() === '' ? undefined : this.checked()),
    (text) => this.repository.inspectIdentifier(text),
  );

  /** The instant a pasted identifier carries, when it carries one. */
  protected readonly created = computed(() => {
    const inspection: IdInspection | null = this.inspection.value();
    return inspection && 'created' in inspection ? readable(inspection.created) : null;
  });

  protected readonly random = computed(() => {
    const inspection: IdInspection | null = this.inspection.value();
    return inspection && 'random' in inspection ? inspection.random : null;
  });

  /** "26 caractères": the length of what was read, as it was pasted. */
  protected readonly characters = computed(() => (this.inspection.answered() ?? '').trim().length);

  protected readonly kindNames = KIND_NAMES;

  readonly result = computed<ToolResult | null>(() => {
    const list = this.list();
    const asked = this.generated.answered();
    return list && asked
      ? {
          title: { key: 'tools.identifiers.noteTitle', params: { kind: KIND_NAMES[asked.kind] } },
          kind: 'snippet',
          language: 'txt',
          content: list,
        }
      : null;
  });

  protected readonly kindName = (kind: IdKind): string => KIND_NAMES[kind];

  sample(): void {
    this.kind.set('uuidV7');
    this.count.set(5);
    this.checked.set(SAMPLE_ID);
  }

  clear(): void {
    this.checked.set('');
  }

  protected regenerate(): void {
    this.draw.update((draw) => draw + 1);
  }

  protected onKeydown(event: KeyboardEvent): void {
    if (event.key.toLowerCase() !== 'r' || event.ctrlKey || event.metaKey || event.altKey) return;
    if (event.defaultPrevented || event.repeat || isTyping(event.target)) return;

    event.preventDefault();
    this.regenerate();
  }

  protected onKind(id: string): void {
    this.kind.set(id as IdKind);
  }

  protected step(by: number): void {
    this.count.update((count) => Math.min(COUNT_BOUNDS.max, Math.max(COUNT_BOUNDS.min, count + by)));
  }

  protected onCount(event: Event): void {
    const count = Number.parseInt((event.target as HTMLInputElement).value, 10);
    if (Number.isFinite(count)) this.count.set(count);
  }

  protected onUppercase(event: Event): void {
    this.uppercase.set((event.target as HTMLInputElement).checked);
  }

  protected onWithoutHyphens(event: Event): void {
    this.hyphens.set(!(event.target as HTMLInputElement).checked);
  }

  protected onNanoLength(event: Event): void {
    const length = Number.parseInt((event.target as HTMLInputElement).value, 10);
    if (Number.isFinite(length)) this.nanoLength.set(length);
  }

  protected onNanoAlphabet(id: string): void {
    this.nanoAlphabet.set(id as NanoAlphabet);
  }

  protected onChecked(event: Event): void {
    this.checked.set((event.target as HTMLInputElement).value);
  }

  protected onRowCopied(row: number, copied: boolean): void {
    if (copied) this.copiedRow.set(row);
    else if (this.copiedRow() === row) this.copiedRow.set(null);
  }
}
