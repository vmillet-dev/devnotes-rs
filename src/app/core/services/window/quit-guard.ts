import { Injectable } from '@angular/core';

/**
 * Work that has to reach Rust before the process ends: a write still waiting behind a debounce.
 * `AppWindowService.quit()` settles it first, since `exit` waits for nothing.
 */
@Injectable({ providedIn: 'root' })
export class QuitGuard {
  private readonly tasks = new Set<() => Promise<void>>();

  /** The returned function takes the task back. */
  register(task: () => Promise<void>): () => void {
    this.tasks.add(task);
    return () => {
      this.tasks.delete(task);
    };
  }

  /** Every task, failures included: a write that fails must not keep the application open. */
  async settle(): Promise<void> {
    await Promise.allSettled([...this.tasks].map((task) => task()));
  }
}
