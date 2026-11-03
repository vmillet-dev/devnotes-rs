import { Injectable, computed, signal } from '@angular/core';

interface StackEntry {
  readonly owner: object;
  readonly rung: number;
}

/**
 * Which modal is in front, so Escape reaches one dialog and not all of them. Ordered
 * by rung and not by arrival: a dialog opened by another one is created second in the
 * DOM but drawn in front, and Escape has to follow what is on screen.
 */
@Injectable({ providedIn: 'root' })
export class DialogStack {
  private readonly entries = signal<readonly StackEntry[]>([]);

  /** Whether anything at all is open — what tells a page its keyboard is taken. */
  readonly hasOpenDialog = computed(() => this.entries().length > 0);

  push(owner: object, rung: number): void {
    this.entries.update((open) => [...open, { owner, rung }]);
  }

  remove(owner: object): void {
    this.entries.update((open) => open.filter((entry) => entry.owner !== owner));
  }

  isFront(owner: object): boolean {
    const open = this.entries();
    const front = open.reduce<StackEntry | null>(
      (best, entry) => (best === null || entry.rung >= best.rung ? entry : best),
      null,
    );

    return front?.owner === owner;
  }
}
