import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  Injector,
  afterNextRender,
  computed,
  inject,
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

  private readonly injector = inject(Injector);
  private readonly rows = viewChildren<ElementRef<HTMLInputElement>>('row');

  protected readonly draft = linkedSignal<number, ChecklistItem[]>({
    source: this.session,
    computation: () => untracked(() => this.items().map((item) => ({ ...item }))),
  });

  protected readonly progress = computed(() => checklistProgress(this.draft()));

  protected readonly dragging = signal<Drag | null>(null);

  /** The row to focus on the next render, set by add and remove, and where its caret goes. */
  private pendingFocus: number | null = null;
  private pendingCaret: number | null = null;

  protected toggle(index: number): void {
    this.draft.update((items) =>
      items.map((item, at) => (at === index ? { ...item, done: !item.done } : item)),
    );
    this.commit();
  }

  protected setText(index: number, text: string): void {
    this.draft.update((items) => items.map((item, at) => (at === index ? { ...item, text } : item)));
  }

  /** Enter cuts the item at the caret: what follows it starts the next one, at its start. */
  protected splitAt(event: Event, index: number): void {
    event.preventDefault();
    const input = event.target as HTMLInputElement;
    const at = input.selectionStart ?? input.value.length;
    const text = input.value;

    this.draft.update((items) => [
      ...items.slice(0, index),
      { ...(items[index] ?? { done: false }), text: text.slice(0, at) },
      { text: text.slice(at), done: false },
      ...items.slice(index + 1),
    ]);
    this.focusNext(index + 1, 0);
    this.commit();
  }

  /**
   * Several lines pasted are several items, from this one on: what stood before the caret
   * heads the first, what stood after it ends the last. One line pastes as it always did.
   */
  protected onPaste(event: ClipboardEvent, index: number): void {
    const lines = (event.clipboardData?.getData('text/plain') ?? '')
      .split(/\r?\n/)
      .filter((line) => line.trim() !== '');
    if (lines.length < 2) return;

    event.preventDefault();
    const input = event.target as HTMLInputElement;
    const before = input.value.slice(0, input.selectionStart ?? input.value.length);
    const after = input.value.slice(input.selectionEnd ?? input.value.length);
    const current = this.draft()[index];
    const [first, ...rest] = lines.map(pastedItem);
    if (!first) return;

    const pasted: ChecklistItem[] = [
      { text: before + first.text, done: before.trim() ? (current?.done ?? false) : first.done },
      ...rest,
    ];
    const end = pasted.length - 1;
    const caret = pasted[end]?.text.length ?? 0;
    pasted[end] = { text: `${pasted[end]?.text ?? ''}${after}`, done: pasted[end]?.done ?? false };

    this.draft.update((items) => [...items.slice(0, index), ...pasted, ...items.slice(index + 1)]);
    this.focusNext(index + end, caret);
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

  /**
   * An empty item goes; at the very start of one with text, it joins the item above, the caret
   * where the two meet. `Event` and not `KeyboardEvent`: a modifier binding is typed `Event`.
   */
  protected onBackspace(event: Event, index: number): void {
    const input = event.target as HTMLInputElement;
    if (input.value === '') {
      if (this.draft().length === 0) return;
      event.preventDefault();
      this.remove(index);
      return;
    }

    const above = this.draft()[index - 1];
    if (!above || input.selectionStart !== 0 || input.selectionEnd !== 0) return;

    event.preventDefault();
    this.draft.update((items) => [
      ...items.slice(0, index - 1),
      { ...above, text: above.text + input.value },
      ...items.slice(index + 1),
    ]);
    this.focusNext(index - 1, above.text.length);
    this.commit();
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
  /** The rows as typed, for a write that cannot wait for the blur. */
  current(): readonly ChecklistItem[] {
    return this.draft().map((item) => ({ ...item }));
  }

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

  private focusNext(index: number, caret: number): void {
    this.pendingFocus = index;
    this.pendingCaret = caret;
  }

  private applyPendingFocus(): void {
    const index = this.pendingFocus;
    const caret = this.pendingCaret;
    this.pendingFocus = null;
    this.pendingCaret = null;
    if (index === null) return;

    // A microtask runs before the zoneless render: the row created would not be there yet.
    afterNextRender(
      () => {
        const row = this.rows()[index]?.nativeElement;
        row?.focus();
        if (row && caret !== null) row.setSelectionRange(caret, caret);
      },
      { injector: this.injector },
    );
  }
}

/** A pasted line read as an item: a list marker dropped, a task box read as done or not. */
function pastedItem(line: string): ChecklistItem {
  const task = /^\s*(?:[-*+]|\d+[.)])?\s*\[([ xX])\]\s+/.exec(line);
  if (task) return { text: line.slice(task[0].length).trimEnd(), done: task[1] !== ' ' };

  return { text: line.replace(/^\s*(?:[-*+]|\d+[.)])\s+/, '').trimEnd(), done: false };
}
