import {
  DestroyRef,
  Directive,
  ElementRef,
  afterNextRender,
  computed,
  inject,
  input,
  signal,
} from '@angular/core';

interface Edges {
  readonly top: number;
  readonly left: number;
  readonly right: number;
}

/** How many items, from the first, stand whole on the box's first line. */
export function wholeCount(box: Edges, items: readonly Edges[]): number {
  let count = 0;
  for (const item of items) {
    // Laid out from the right: a first item wider than the row overflows on its left.
    if (item.top - box.top >= 1 || item.left < box.left - 0.5 || item.right > box.right + 0.5) break;
    count += 1;
  }
  return count;
}

/** One observer for every row of items on screen: a canvas holds hundreds of cards. */
const watched = new WeakMap<Element, () => void>();
let shared: ResizeObserver | null = null;
function observer(): ResizeObserver | null {
  if (typeof ResizeObserver === 'undefined') return null;
  shared ??= new ResizeObserver((entries) => entries.forEach((entry) => watched.get(entry.target)?.()));
  return shared;
}

/**
 * A row of items that wraps onto lines it does not show, so an item that does not hold is left
 * out whole, never cut mid-word: those past the first line, and a first one wider than the row,
 * are hidden. \`shown\` says how many stand, for a « +N » beside the row.
 */
@Directive({
  selector: '[appWholeItems]',
  exportAs: 'appWholeItems',
})
export class WholeItemsDirective {
  /** How many items the row holds: measured again when it changes. */
  readonly count = input.required<number>({ alias: 'appWholeItems' });

  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);
  private readonly measured = signal<number | null>(null);

  readonly shown = computed(() => Math.min(this.measured() ?? this.count(), this.count()));

  constructor() {
    const destroyRef = inject(DestroyRef);

    afterNextRender(() => {
      const resized = observer();
      if (!resized) return;
      const row = this.host.nativeElement;
      const changed = new MutationObserver(() => this.measure());
      changed.observe(row, { childList: true, subtree: true, characterData: true });
      watched.set(row, () => this.measure());
      resized.observe(row);
      this.measure();
      destroyRef.onDestroy(() => {
        resized.unobserve(row);
        watched.delete(row);
        changed.disconnect();
      });
    });
  }

  private measure(): void {
    const row = this.host.nativeElement;
    // A hidden page measures nothing: it keeps the answer it had when last shown.
    if (row.getClientRects().length === 0) return;
    const items = [...row.children].filter((child): child is HTMLElement => child instanceof HTMLElement);
    const shown = wholeCount(
      row.getBoundingClientRect(),
      items.map((item) => item.getBoundingClientRect()),
    );
    items.forEach((item, index) => (item.style.visibility = index < shown ? '' : 'hidden'));
    this.measured.set(shown);
  }
}
