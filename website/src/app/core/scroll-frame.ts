import { Injectable } from '@angular/core';

/**
 * One passive scroll listener for the whole page, and at most one pass a frame: the hero
 * board, the windows that settle as they rise and the progress line all read the same
 * geometry, and three listeners would each force their own layout.
 */
@Injectable({ providedIn: 'root' })
export class ScrollFrame {
  private readonly listeners = new Set<() => void>();
  private attached = false;
  private pending = false;

  /** Browser only, from `afterNextRender`. Runs the listener now, then on every frame the page moves. */
  watch(listener: () => void): () => void {
    this.listeners.add(listener);
    this.attach();
    listener();
    return () => this.listeners.delete(listener);
  }

  private attach(): void {
    if (this.attached) return;
    this.attached = true;

    const request = () => {
      if (this.pending) return;
      this.pending = true;
      requestAnimationFrame(() => {
        this.pending = false;
        this.listeners.forEach((listener) => listener());
      });
    };
    addEventListener('scroll', request, { passive: true });
    addEventListener('resize', request);
  }
}
