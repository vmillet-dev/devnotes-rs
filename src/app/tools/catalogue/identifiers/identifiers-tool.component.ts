import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { ToolsRepository } from '@core/data/tools.repository';
import { IdInspection, IdKind, NanoAlphabet } from '@core/model/tool-answers.model';
import { liveResult } from '@core/services/tools/live-result';
import { Tool, ToolResult } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import { ChoiceMenuComponent, ChoiceOption } from '@shared/controls/choice-menu/choice-menu.component';
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

const KIND_OPTIONS: readonly ChoiceOption[] = Object.entries(KIND_NAMES).map(([id, name]) => ({ id, name }));

/** The kinds whose letters carry no meaning in their case: hexadecimal and Crockford's base 32. */
const CASELESS: ReadonlySet<IdKind> = new Set(['uuidV4', 'uuidV7', 'ulid', 'objectId']);

const ALPHABETS: readonly Segment[] = (['urlSafe', 'alphanumeric', 'hexLower', 'digits'] as const).map(
  (id) => ({
    id,
    labelKey: `tools.identifiers.alphabets.${id}`,
  }),
);

/** "2024-09-25 23:05:01.488 UTC": the instant as Rust wrote it, read in any language. */
function readable(iso: string | null): string | null {
  return iso ? iso.replace('T', ' ').replace('Z', ' UTC') : null;
}

@Component({
  selector: 'app-identifiers-tool',
  imports: [ChoiceMenuComponent, CopyValueComponent, SegmentedChoiceComponent, TranslocoPipe],
  templateUrl: './identifiers-tool.component.html',
  styleUrl: './identifiers-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class IdentifiersToolComponent implements Tool {
  private readonly repository = inject(ToolsRepository);

  protected readonly kind = toolState<IdKind>('identifiers.kind', 'uuidV4');
  protected readonly count = toolState('identifiers.count', 5);
  protected readonly uppercase = toolState('identifiers.uppercase', false);
  protected readonly nanoLength = toolState('identifiers.nanoLength', 21);
  protected readonly nanoAlphabet = toolState<NanoAlphabet>('identifiers.nanoAlphabet', 'urlSafe');
  protected readonly checked = toolState('identifiers.checked', '');

  private readonly draw = signal(0);

  protected readonly kinds = KIND_OPTIONS;
  protected readonly alphabets = ALPHABETS;

  protected readonly caseless = computed(() => CASELESS.has(this.kind()));

  protected readonly generated = liveResult(
    () => ({
      kind: this.kind(),
      count: this.count(),
      uppercase: this.uppercase(),
      nanoLength: this.nanoLength(),
      nanoAlphabet: this.nanoAlphabet(),
      draw: this.draw(),
    }),
    ({ draw: _draw, ...request }) => this.repository.generateIdentifiers(request),
  );

  protected readonly list = computed(() => (this.generated.value() ?? []).join('\n'));

  protected readonly inspection = liveResult(
    () => (this.checked().trim() === '' ? undefined : this.checked()),
    (text) => this.repository.inspectIdentifier(text),
  );

  /** The instant a pasted identifier carries, when it carries one. */
  protected readonly created = computed(() => {
    const inspection: IdInspection | null = this.inspection.value();
    return inspection && 'created' in inspection ? readable(inspection.created) : null;
  });

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

  clear(): void {
    this.checked.set('');
  }

  protected regenerate(): void {
    this.draw.update((draw) => draw + 1);
  }

  protected onKind(id: string | null): void {
    if (id !== null) this.kind.set(id as IdKind);
  }

  protected onCount(event: Event): void {
    const count = Number.parseInt((event.target as HTMLInputElement).value, 10);
    if (Number.isFinite(count)) this.count.set(count);
  }

  protected onUppercase(event: Event): void {
    this.uppercase.set((event.target as HTMLInputElement).checked);
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
}
