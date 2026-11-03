import { DestroyRef, Injectable, Signal, inject, signal } from '@angular/core';

/** Short enough for "just now" to become "1 min ago" in time. */
export const CLOCK_TICK_MS = 30_000;

/**
 * ⚠️ A `new Date()` read inside a `computed()` freezes it: it depends on no signal
 * representing time, so a card shows "4 min ago" forever.
 */
@Injectable({ providedIn: 'root' })
export class ClockService {
  private readonly _now = signal(new Date());

  readonly now: Signal<Date> = this._now.asReadonly();

  constructor() {
    const intervalId = setInterval(() => this._now.set(new Date()), CLOCK_TICK_MS);
    inject(DestroyRef).onDestroy(() => clearInterval(intervalId));
  }
}
