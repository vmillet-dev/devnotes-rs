import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  computed,
  input,
  linkedSignal,
  output,
  signal,
  untracked,
  viewChildren,
} from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { ChecklistItem, checklistProgress } from '@core/model/checklist.model';

interface Drag {
  readonly from: number;
  readonly to: number;
}

/**
 * HTML5 drag and drop does not work here: Tauri's `dragDropEnabled` is `true` — it is
 * what delivers dropped files to `FileDropService` — so the WebView never sees
 * `dragstart` or `drop`. Reordering is pointer events instead, and turning the flag off
 * would break attachments.
 */
@Component({
  selector: 'app-checklist-editor',
  imports: [TranslocoPipe],
  templateUrl: './checklist-editor.component.html',
  styleUrl: './checklist-editor.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ChecklistEditorComponent {
  readonly items = input.required<readonly ChecklistItem[]>();
  /**
   * The editor session, not the note's id: materialising a draft changes that id for
   * the same note, and a draft keyed on it is replayed over what was just typed.
   */
  readonly session = input.required<number>();

  readonly itemsChanged = output<readonly ChecklistItem[]>();

  private readonly rows = viewChildren<ElementRef<HTMLInputElement>>('row');

  protected readonly draft = linkedSignal<number, ChecklistItem[]>({
    source: this.session,
    computation: () => untracked(() => this.items().map((item) => ({ ...item }))),
  });

  protected readonly progress = computed(() => checklistProgress(this.draft()));

  protected readonly dragging = signal<Drag | null>(null);

  /** The row to focus on the next render, set by add and remove. */
  private pendingFocus: number | null = null;

  protected toggle(index: number): void {
    this.draft.update((items) =>
      items.map((item, at) => (at === index ? { ...item, done: !item.done } : item)),
    );
    this.commit();
  }

  protected setText(index: number, text: string): void {
    this.draft.update((items) => items.map((item, at) => (at === index ? { ...item, text } : item)));
  }

  protected insertAfter(index: number): void {
    this.draft.update((items) => [
      ...items.slice(0, index + 1),
      { text: '', done: false },
      ...items.slice(index + 1),
    ]);
    this.pendingFocus = index + 1;
    this.commit();
  }

  protected append(): void {
    this.draft.update((items) => [...items, { text: '', done: false }]);
    this.pendingFocus = this.draft().length - 1;
    this.commit();
  }

  protected remove(index: number): void {
    this.draft.update((items) => items.filter((_, at) => at !== index));
    // The previous row, not the next: that is where the cursor was.
    this.pendingFocus = Math.max(0, index - 1);
    this.commit();
  }

  /** `Event` and not `KeyboardEvent`: a modifier binding is typed `Event` by the template compiler. */
  protected onBackspace(event: Event, index: number): void {
    const input = event.target as HTMLInputElement;
    if (input.value !== '' || this.draft().length === 0) return;

    event.preventDefault();
    this.remove(index);
  }

  protected move(from: number, to: number): void {
    const items = this.draft();
    if (to < 0 || to >= items.length || from === to) return;

    const reordered = [...items];
    const [moved] = reordered.splice(from, 1);
    if (!moved) return;

    reordered.splice(to, 0, moved);

    this.draft.set(reordered);
    this.pendingFocus = to;
    this.commit();
  }

  /**
   * `setPointerCapture` keeps the events on the handle when the cursor leaves the row.
   * Optional-chained because jsdom lacks it and must not fail the drag.
   */
  protected onPointerDown(event: PointerEvent, index: number): void {
    if (event.button !== 0) return;

    event.preventDefault();
    (event.target as HTMLElement).setPointerCapture?.(event.pointerId);
    this.dragging.set({ from: index, to: index });
  }

  protected onPointerMove(event: PointerEvent): void {
    const drag = this.dragging();
    if (!drag) return;

    const to = this.rowIndexAt(event.clientY);
    if (to !== null && to !== drag.to) {
      this.dragging.set({ from: drag.from, to });
    }
  }

  protected onPointerUp(event: PointerEvent): void {
    const drag = this.dragging();
    this.dragging.set(null);
    if (!drag) return;

    (event.target as HTMLElement).releasePointerCapture?.(event.pointerId);
    this.move(drag.from, drag.to);
  }

  protected displayIndex(index: number): number {
    const drag = this.dragging();
    if (!drag) return index;

    if (index === drag.from) return drag.to;
    if (drag.from < index && index <= drag.to) return index - 1;
    if (drag.to <= index && index < drag.from) return index + 1;

    return index;
  }

  /**
   * Called on `blur` and by the editor before it closes: Escape, the backdrop and
   * the close button produce no `blur`, so the last line typed would be lost.
   */
  commit(): void {
    this.itemsChanged.emit(this.draft().map((item) => ({ ...item })));
    this.applyPendingFocus();
  }

  private rowIndexAt(clientY: number): number | null {
    const rows = this.rows();
    for (const [index, row] of rows.entries()) {
      const box = row.nativeElement.getBoundingClientRect();
      if (clientY < box.bottom) return index;
    }

    return rows.length > 0 ? rows.length - 1 : null;
  }

  private applyPendingFocus(): void {
    const index = this.pendingFocus;
    this.pendingFocus = null;
    if (index === null) return;

    // After the created or moved row renders: the signal has just triggered it.
    queueMicrotask(() => this.rows()[index]?.nativeElement.focus());
  }
}
