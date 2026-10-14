/**
 * The page is rendered twice: once by Node at build time, where neither `window` nor its
 * observers exist, then in the browser. Everything that measures or animates runs in
 * `afterNextRender`, and asks these first — jsdom, under the tests, lacks them too.
 */
export function prefersReducedMotion(): boolean {
  return matches('(prefers-reduced-motion: reduce)');
}

export function matches(query: string): boolean {
  return typeof matchMedia === 'function' && matchMedia(query).matches;
}

export function canObserve(): boolean {
  return typeof IntersectionObserver === 'function';
}

export const clamp = (value: number, low = 0, high = 1): number => Math.min(high, Math.max(low, value));

/** Where `t` stands between `from` and `to`, as 0…1. */
export const part = (t: number, from: number, to: number): number => clamp((t - from) / (to - from));

/** The first `t` of a text, as if typed. */
export const typed = (text: string, t: number): string => text.slice(0, Math.round(text.length * clamp(t)));

export const ease = (t: number): number => (t < 0.5 ? 2 * t * t : 1 - (-2 * t + 2) ** 2 / 2);

export const sleep = (ms: number): Promise<void> => new Promise((done) => setTimeout(done, ms));

/** A snippet's `{{field}}`, written where a template would read the braces as its own. */
export const field = (name: string): string => `{{${name}}}`;
