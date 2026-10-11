import { ChangeDetectionStrategy, Component, input, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';

/** What one arrow key moves the edge by: a nudge, not a jump. */
const RESIZE_STEP_PX = 16;

export interface RailBounds {
  readonly min: number;
  readonly max: number;
}

/**
 * What the rail of each area is drawn in: its width, its background, the line on its right and
 * the edge that drags it. One piece for the three, so that going from one area to another moves
 * nothing — and so that each can be widened, where only the notes' could. It takes a width and
 * emits the new one; whoever holds it stores it.
 */
@Component({
  selector: 'app-rail-frame',
  imports: [TranslocoPipe],
  templateUrl: './rail-frame.component.html',
  styleUrl: './rail-frame.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { '[style.width.px]': 'width()' },
})
export class RailFrameComponent {
  readonly width = input.required<number>();
  readonly bounds = input.required<RailBounds>();
  /** One a page: the notes' rail stays in the document behind the other areas. */
  readonly edgeTestId = input('rail-edge');

  readonly widthChanged = output<number>();

  /**
   * Pointer events, like every other drag in this application: HTML5 drag and drop
   * does not work in this WebView and cannot be turned on. The edge is a slider, so
   * the arrow keys move it too — the same rule the board's grips follow.
   */
  protected startResize(event: PointerEvent): void {
    if (event.button !== 0) return;

    event.preventDefault();
    const handle = event.target as HTMLElement;
    handle.setPointerCapture?.(event.pointerId);
  }

  protected onResizeMove(event: PointerEvent): void {
    const handle = event.target as HTMLElement;
    if (!handle.hasPointerCapture?.(event.pointerId)) return;

    // Measured from the rail's own left edge, so a window that is not at x=0 is no trap.
    const left = handle.parentElement?.getBoundingClientRect().left ?? 0;
    this.emitWidth(event.clientX - left);
  }

  protected endResize(event: PointerEvent): void {
    (event.target as HTMLElement).releasePointerCapture?.(event.pointerId);
  }

  protected onResizeKey(event: KeyboardEvent): void {
    const step =
      event.key === 'ArrowRight' ? RESIZE_STEP_PX : event.key === 'ArrowLeft' ? -RESIZE_STEP_PX : 0;
    if (step === 0) return;

    event.preventDefault();
    this.emitWidth(this.width() + step);
  }

  private emitWidth(width: number): void {
    const { min, max } = this.bounds();
    const clamped = Math.min(max, Math.max(min, Math.round(width)));
    if (clamped !== this.width()) {
      this.widthChanged.emit(clamped);
    }
  }
}
