import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  computed,
  input,
  output,
  signal,
  viewChild,
} from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { RunnerRow } from '@core/model/http-runner.model';
import { requestBadge, shortBadge } from '@core/model/http.model';

/** A drag under way: the row taken, and where it would land. */
interface Dragging {
  readonly from: number;
  readonly to: number;
}

/**
 * The run's order: dragged by its grip, or moved with Alt + ↑ ↓ on a row; Space leaves a row
 * out. Pointer events, as every drag of this application: HTML5's does not reach the WebView.
 */
@Component({
  selector: 'app-runner-order',
  imports: [TranslocoPipe],
  templateUrl: './runner-order.component.html',
  styleUrl: './runner-order.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class RunnerOrderComponent {
  readonly rows = input.required<readonly RunnerRow[]>();
  readonly disabled = input(false);
  readonly moved = output<{ readonly from: number; readonly to: number }>();
  readonly toggled = output<number>();

  private readonly list = viewChild.required<ElementRef<HTMLElement>>('list');
  protected readonly dragging = signal<Dragging | null>(null);
  protected readonly running = computed(() => this.rows().filter((row) => row.enabled).length);

  protected badge(row: RunnerRow): string {
    return shortBadge(requestBadge(row));
  }

  protected onKey(index: number, event: KeyboardEvent): void {
    if (this.disabled()) return;
    if (event.key === ' ') {
      event.preventDefault();
      this.toggled.emit(index);
    } else if (event.altKey && (event.key === 'ArrowUp' || event.key === 'ArrowDown')) {
      event.preventDefault();
      const to = index + (event.key === 'ArrowUp' ? -1 : 1);
      if (to < 0 || to >= this.rows().length) return;
      this.moved.emit({ from: index, to });
      // The row keeps the focus where it went.
      queueMicrotask(() => this.rowAt(to)?.focus());
    }
  }

  protected grab(index: number, event: PointerEvent): void {
    if (this.disabled() || event.button !== 0) return;
    event.preventDefault();
    (event.currentTarget as HTMLElement).setPointerCapture?.(event.pointerId);
    this.dragging.set({ from: index, to: index });
  }

  protected drag(event: PointerEvent): void {
    const dragging = this.dragging();
    if (dragging === null) return;
    this.dragging.set({ ...dragging, to: this.indexAt(event.clientY) });
  }

  protected drop(event: PointerEvent): void {
    const dragging = this.dragging();
    (event.currentTarget as HTMLElement).releasePointerCapture?.(event.pointerId);
    this.dragging.set(null);
    if (dragging !== null && dragging.to !== dragging.from) this.moved.emit(dragging);
  }

  private indexAt(y: number): number {
    const rows = [
      ...this.list().nativeElement.querySelectorAll<HTMLElement>('[data-testid="http-runner-row"]'),
    ];
    const found = rows.findIndex((row) => y < row.getBoundingClientRect().bottom);
    return found < 0 ? rows.length - 1 : found;
  }

  private rowAt(index: number): HTMLElement | undefined {
    return this.list().nativeElement.querySelectorAll<HTMLElement>('[data-testid="http-runner-row"]')[index];
  }
}
