import { Signal, computed, effect, inject, linkedSignal, resource, signal, untracked } from '@angular/core';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { debounced } from '@core/services/time/debounce';

/** Long enough to let a word be typed, short enough to read as live. */
export const LIVE_RESULT_DEBOUNCE_MS = 150;

export interface LiveResult<T> {
  /** The last answer, kept on screen while the next is computed; `null` once nothing is asked. */
  readonly value: Signal<T | null>;
  readonly pending: Signal<boolean>;
}

function sameJson(a: unknown, b: unknown): boolean {
  return a === b || JSON.stringify(a) === JSON.stringify(b);
}

/**
 * Every tool's round trip to Rust as its inputs change: debounced, compared by value — a fresh
 * literal would ask again for nothing — and the previous answer kept until the next lands.
 * `undefined` asks nothing. A tool's own failure is part of its answer; what lands here is the
 * unexpected, reported like any other. Built in an injection context.
 */
export function liveResult<P, R>(
  request: () => P | undefined,
  load: (request: P) => Promise<R>,
  delayMs = LIVE_RESULT_DEBOUNCE_MS,
): LiveResult<R> {
  const notifier = inject(ErrorNotifier);
  const asked = computed(request, { equal: sameJson });
  // The first request goes at once: a tool found again shows its answer without a wait.
  const settled = signal<P | undefined>(untracked(asked), { equal: sameJson });
  const settle = debounced((next: P | undefined) => settled.set(next), delayMs);

  effect(() => {
    const next = asked();
    if (next === undefined) {
      settle.cancel();
      settled.set(undefined);
    } else {
      settle(next);
    }
  });

  const answer = resource({ params: () => settled(), loader: ({ params }) => load(params) });

  effect(() => {
    if (answer.status() === 'error') {
      notifier.reportFailure('errors.toolFailed', answer.error());
    }
  });

  const value = linkedSignal<{ asked: boolean; fresh: R | undefined }, R | null>({
    source: () => ({
      asked: settled() !== undefined,
      fresh: answer.hasValue() ? answer.value() : undefined,
    }),
    computation: ({ asked: isAsked, fresh }, previous) =>
      isAsked ? (fresh ?? previous?.value ?? null) : null,
  });

  return {
    value,
    pending: computed(() => !sameJson(asked(), settled()) || answer.isLoading()),
  };
}
