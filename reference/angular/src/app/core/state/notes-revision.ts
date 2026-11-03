import { Injectable, Signal, signal } from '@angular/core';

/**
 * Bumped by a store that writes notes without knowing `NotesStore`, which would close a cycle:
 * the canvas and the board read it among their query parameters and re-run on their own.
 */
@Injectable({ providedIn: 'root' })
export class NotesRevision {
  private readonly counter = signal(0);

  readonly current: Signal<number> = this.counter.asReadonly();

  bump(): void {
    this.counter.update((revision) => revision + 1);
  }
}
