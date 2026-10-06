import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  computed,
  inject,
  signal,
  viewChild,
} from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { ToolsRepository } from '@core/data/tools.repository';
import { JsonChange, JsonDiffRow, JsonDiffSide, ValueSummary } from '@core/model/tool-answers.model';
import { ClipboardService } from '@core/services/clipboard/clipboard.service';
import { TranslationRef } from '@core/services/i18n/translation-ref.model';
import { liveResult } from '@core/services/tools/live-result';
import { Tool, ToolAction, ToolResult } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import {
  Segment,
  SegmentedChoiceComponent,
} from '@shared/controls/segmented-choice/segmented-choice.component';
import { JsonNotePickerComponent, PickedNote } from './note-picker/json-note-picker.component';

type Layout = 'split' | 'unified';

const LAYOUTS: readonly Segment[] = (['split', 'unified'] as const).map((id) => ({
  id,
  labelKey: `tools.diff.structured.layouts.${id}`,
}));

const MARKS: Record<JsonChange['kind'], string> = { added: '+', removed: '−', modified: '~', reordered: '~' };

/** One line of the unified view: the side-by-side rows, read one after the other. */
interface UnifiedLine {
  readonly mark: '' | '+' | '−';
  readonly kind: 'same' | 'added' | 'removed';
  readonly number: number;
  readonly text: string;
  readonly row: number;
}

function unified(rows: readonly JsonDiffRow[]): UnifiedLine[] {
  return rows.flatMap((row, index): UnifiedLine[] => {
    const lines: UnifiedLine[] = [];
    if (row.kind === 'same' && row.left) {
      return [{ mark: '', kind: 'same', number: row.left.number, text: row.left.text, row: index }];
    }
    if (row.left)
      lines.push({ mark: '−', kind: 'removed', number: row.left.number, text: row.left.text, row: index });
    if (row.right)
      lines.push({ mark: '+', kind: 'added', number: row.right.number, text: row.right.text, row: index });
    return lines;
  });
}

const SAMPLE = {
  a: `{
  "name": "devnotes",
  "version": "0.9.1",
  "features": ["notes", "tools"],
  "window": { "width": 1280, "height": 800 }
}`,
  b: `{
  "name": "devnotes",
  "version": "0.9.2",
  "features": ["notes", "tools", "samples"],
  "window": { "width": 1440, "height": 800 }
}`,
};

@Component({
  selector: 'app-structured-diff',
  imports: [JsonNotePickerComponent, SegmentedChoiceComponent, TranslocoPipe],
  templateUrl: './structured-diff.component.html',
  styleUrl: './structured-diff.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class StructuredDiffComponent implements Tool {
  private readonly repository = inject(ToolsRepository);
  private readonly clipboard = inject(ClipboardService);

  protected readonly a = toolState('diff.a', '');
  protected readonly b = toolState('diff.b', '');
  protected readonly aName = toolState<string | null>('diff.aName', null);
  protected readonly bName = toolState<string | null>('diff.bName', null);
  protected readonly ignoreKeyOrder = toolState('diff.ignoreKeyOrder', true);
  protected readonly ignoreWhitespace = toolState('diff.ignoreWhitespace', true);
  protected readonly layout = toolState<Layout>('diff.structuredLayout', 'split');

  protected readonly picking = signal<JsonDiffSide | null>(null);
  protected readonly selectedPath = signal<string | null>(null);

  private readonly rowsBox = viewChild<ElementRef<HTMLElement>>('rowsBox');

  protected readonly layouts = LAYOUTS;
  protected readonly marks = MARKS;

  protected readonly answer = liveResult(
    () =>
      this.a().trim() === '' || this.b().trim() === ''
        ? undefined
        : {
            a: this.a(),
            b: this.b(),
            ignoreKeyOrder: this.ignoreKeyOrder(),
            ignoreWhitespace: this.ignoreWhitespace(),
          },
    (request) => this.repository.diffJson(request),
    250,
  );

  protected readonly compared = computed(() => {
    const answer = this.answer.value();
    return answer?.kind === 'compared' ? answer : null;
  });

  protected readonly unreadable = computed(() => {
    const answer = this.answer.value();
    return answer?.kind === 'unreadable' ? answer : null;
  });

  protected readonly unifiedLines = computed(() => unified(this.compared()?.rows ?? []));

  /** The patch has operations: there is something to copy or to keep. */
  private readonly patch = computed(() => {
    const compared = this.compared();
    return compared && compared.changes.some((change) => change.kind !== 'reordered') ? compared.patch : null;
  });

  readonly actions = computed<readonly ToolAction[]>(() => [
    { id: 'swap', labelKey: 'tools.diff.swap', disabled: false, run: () => this.swap() },
    {
      id: 'copy-patch',
      labelKey: 'tools.diff.structured.copyPatch',
      disabled: this.patch() === null,
      run: () => void this.copyPatch(),
    },
  ]);

  readonly result = computed<ToolResult | null>(() => {
    const patch = this.patch();
    return patch
      ? {
          title: {
            key: 'tools.diff.structured.noteTitle',
            params: { a: this.aName() ?? 'A', b: this.bName() ?? 'B' },
          },
          kind: 'snippet',
          language: 'json',
          content: patch,
        }
      : null;
  });

  sample(): void {
    this.a.set(SAMPLE.a);
    this.b.set(SAMPLE.b);
    this.aName.set(null);
    this.bName.set(null);
  }

  clear(): void {
    this.a.set('');
    this.b.set('');
    this.aName.set(null);
    this.bName.set(null);
    this.selectedPath.set(null);
  }

  protected swap(): void {
    const [a, b, aName, bName] = [this.a(), this.b(), this.aName(), this.bName()];
    this.a.set(b);
    this.b.set(a);
    this.aName.set(bName);
    this.bName.set(aName);
  }

  protected async copyPatch(): Promise<void> {
    const patch = this.patch();
    if (patch !== null) {
      await this.clipboard.copy(patch);
    }
  }

  protected onInput(side: JsonDiffSide, event: Event): void {
    const text = (event.target as HTMLTextAreaElement).value;
    if (side === 'a') {
      this.a.set(text);
      this.aName.set(null);
    } else {
      this.b.set(text);
      this.bName.set(null);
    }
  }

  protected onPicked(note: PickedNote): void {
    if (this.picking() === 'a') {
      this.a.set(note.content);
      this.aName.set(note.title);
    } else {
      this.b.set(note.content);
      this.bName.set(note.title);
    }
    this.picking.set(null);
  }

  protected onOption(which: 'order' | 'spaces', event: Event): void {
    const checked = (event.target as HTMLInputElement).checked;
    (which === 'order' ? this.ignoreKeyOrder : this.ignoreWhitespace).set(checked);
  }

  protected onLayout(id: string): void {
    this.layout.set(id as Layout);
  }

  /** Both sides scroll together, so bringing the first row of a change into view shows both. */
  protected show(change: JsonChange): void {
    this.selectedPath.set(change.path);
    const rows = this.compared()?.rows ?? [];
    const index = rows.findIndex((row) => row.path === change.path);
    const row = this.rowsBox()?.nativeElement.querySelector(`[data-row="${index}"]`);
    row?.scrollIntoView?.({ block: 'center' });
  }

  /** A value as the list shows it: itself when short, its size — translated — otherwise. */
  protected described(summary: ValueSummary): TranslationRef {
    switch (summary.kind) {
      case 'scalar':
        return { key: 'tools.diff.structured.scalar', params: { text: summary.text } };
      case 'object':
        return { key: 'tools.diff.structured.object', params: { count: summary.keys } };
      case 'array':
        return { key: 'tools.diff.structured.array', params: { count: summary.items } };
    }
  }
}
