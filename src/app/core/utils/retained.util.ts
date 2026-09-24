import { Resource, Signal, linkedSignal } from '@angular/core';

/**
 * The last value `resource` loaded, kept through its reloads so the view does not blank.
 * ⚠️ A `linkedSignal` retains only what it has seen: read everything through this, and read it
 * first in an `&&`, where a short-circuit would skip it.
 */
export function retained<T>(resource: Resource<T | undefined>): Signal<T | null> {
  return linkedSignal<T | undefined, T | null>({
    source: () => (resource.hasValue() ? resource.value() : undefined),
    computation: (fresh, previous) => fresh ?? previous?.value ?? null,
  });
}
