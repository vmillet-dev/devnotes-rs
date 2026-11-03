import { ChangeDetectionStrategy, Component, ElementRef, inject, input, output } from '@angular/core';

/** What one arrow key moves the line by, in points of the box's height: a nudge, not a jump. */
const STEP = 4;

/**
 * The line between two panes in one box, dragged to share it between them: stacked, their
 * height; side by side (`orientation: 'vertical'`, the line standing up), their width. It sits
 * between the two in the template: it measures its parent and the element before it, takes the
 * share that is stored and emits the new one; whoever holds it stores it.
 *
 * Pointer events, like every other drag in this application — HTML5 drag and drop does not
 * reach this WebView — and a separator that takes the focus, so the arrows move it too.
 * ⚠️ `0` is no share at all: the upper pane is left to what it holds, and that is where a
 * double-click, or Enter, puts the line back.
 */
@Component({
  selector: 'app-split-handle',
  template: '',
  styleUrl: './split-handle.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: {
    role: 'separator',
    tabindex: '0',
    '[attr.aria-orientation]': 'orientation()',
    '[class.vertical]': "orientation() === 'vertical'",
    'aria-valuemin': '0',
    'aria-valuemax': '100',
    '[attr.aria-valuenow]': 'share() > 0 ? share() : null',
    '[attr.aria-valuetext]': 'valueText()',
    '[attr.aria-label]': 'label()',
    '[attr.title]': 'label()',
    '[attr.data-testid]': 'testId()',
    '[attr.data-share]': 'share()',
    '(pointerdown)': 'start($event)',
    '(pointermove)': 'move($event)',
    '(pointerup)': 'end($event)',
    '(pointercancel)': 'end($event)',
    '(dblclick)': 'reset()',
    '(keydown)': 'onKey($event)',
  },
})
export class SplitHandleComponent {
  /** The line's own direction: `vertical` stands between panes side by side. */
  readonly orientation = input<'horizontal' | 'vertical'>('horizontal');
  /** The upper (or left) pane's share of the box, in percent; `0` while it is left to its content. */
  readonly share = input.required<number>();
  /** What neither pane may go under, in pixels: neither is dragged out of reach. Upper is left. */
  readonly upperFloor = input.required<number>();
  readonly lowerFloor = input.required<number>();
  readonly label = input.required<string>();
  /** The share in words, « automatique » included: what a screen reader announces. */
  readonly valueText = input.required<string>();
  readonly testId = input('split-handle');

  readonly shareChanged = output<number>();

  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);

  protected start(event: PointerEvent): void {
    if (event.button !== 0) return;
    event.preventDefault();
    this.host.nativeElement.setPointerCapture?.(event.pointerId);
  }

  protected move(event: PointerEvent): void {
    if (!this.host.nativeElement.hasPointerCapture?.(event.pointerId)) return;
    const box = this.box();
    if (box === null) return;
    // Measured from the box's own edge, so a window that is not at 0 is no trap.
    const along = this.vertical() ? event.clientX - box.left : event.clientY - box.top;
    this.emit((along / this.size(box)) * 100, box);
  }

  protected end(event: PointerEvent): void {
    this.host.nativeElement.releasePointerCapture?.(event.pointerId);
  }

  protected reset(): void {
    if (this.share() !== 0) this.shareChanged.emit(0);
  }

  protected onKey(event: KeyboardEvent): void {
    const box = this.box();
    if (box === null) return;
    const { min, max } = this.bounds(box);
    const [back, forth] = this.vertical() ? ['ArrowLeft', 'ArrowRight'] : ['ArrowUp', 'ArrowDown'];
    const to =
      event.key === back
        ? this.current(box) - STEP
        : event.key === forth
          ? this.current(box) + STEP
          : event.key === 'Home'
            ? min
            : event.key === 'End'
              ? max
              : null;
    if (to !== null) {
      event.preventDefault();
      this.emit(to, box);
    } else if (event.key === 'Enter') {
      event.preventDefault();
      this.reset();
    }
  }

  /** Where the line stands now: stored, or measured while the upper pane is left to its content. */
  private current(box: DOMRect): number {
    if (this.share() > 0) return this.share();
    const before = this.host.nativeElement.previousElementSibling?.getBoundingClientRect();
    const upper = (this.vertical() ? before?.width : before?.height) ?? 0;
    return (upper / this.size(box)) * 100;
  }

  private bounds(box: DOMRect): { min: number; max: number } {
    // The line itself takes no height from either pane: it is drawn over their border.
    const size = this.size(box);
    const min = (this.upperFloor() / size) * 100;
    const max = ((size - this.lowerFloor()) / size) * 100;
    // A box too short for both floors: the upper one wins, the lower pane scrolls.
    return { min, max: Math.max(min, max) };
  }

  private emit(share: number, box: DOMRect): void {
    const { min, max } = this.bounds(box);
    const clamped = Math.round(Math.min(max, Math.max(min, share)) * 10) / 10;
    if (clamped !== this.share()) this.shareChanged.emit(clamped);
  }

  private vertical(): boolean {
    return this.orientation() === 'vertical';
  }

  private size(box: DOMRect): number {
    return this.vertical() ? box.width : box.height;
  }

  private box(): DOMRect | null {
    const box = this.host.nativeElement.parentElement?.getBoundingClientRect();
    return box && this.size(box) > 0 ? box : null;
  }
}
