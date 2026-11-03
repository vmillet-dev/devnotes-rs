import { DestroyRef, Injector, runInInjectionContext } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { SEARCH_DEBOUNCE_MS, debounced } from './debounce';

describe('debounced', () => {
  let injector: Injector;
  let destroy: () => void;

  beforeEach(() => {
    // Only the timers: a faked `requestAnimationFrame` hangs Angular's zoneless scheduler.
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({});
    injector = TestBed.inject(Injector);
    destroy = () => TestBed.resetTestingModule();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  function build<T>(action: (value: T) => void, delay = SEARCH_DEBOUNCE_MS) {
    return runInInjectionContext(injector, () => debounced(action, delay));
  }

  it('waits out the delay before acting', () => {
    const seen: string[] = [];
    const call = build<string>((value) => seen.push(value));

    call('a');
    expect(seen).toEqual([]);

    vi.advanceTimersByTime(SEARCH_DEBOUNCE_MS);
    expect(seen).toEqual(['a']);
  });

  /** The point of it: one call per keystroke would be one IPC round trip per keystroke. */
  it('keeps only the last value of a burst', () => {
    const seen: string[] = [];
    const call = build<string>((value) => seen.push(value));

    call('p');
    call('pr');
    call('pro');
    vi.advanceTimersByTime(SEARCH_DEBOUNCE_MS);

    expect(seen).toEqual(['pro']);
  });

  it('restarts the delay on every call rather than acting on a schedule', () => {
    const seen: string[] = [];
    const call = build<string>((value) => seen.push(value));

    call('a');
    vi.advanceTimersByTime(SEARCH_DEBOUNCE_MS - 20);
    call('b');
    vi.advanceTimersByTime(SEARCH_DEBOUNCE_MS - 20);

    expect(seen).toEqual([]);
    vi.advanceTimersByTime(20);
    expect(seen).toEqual(['b']);
  });

  it('drops a call still waiting, and the next one starts fresh', () => {
    const seen: string[] = [];
    const call = build<string>((value) => seen.push(value));

    call('a');
    call.cancel();
    vi.advanceTimersByTime(SEARCH_DEBOUNCE_MS);
    expect(seen).toEqual([]);

    call('b');
    vi.advanceTimersByTime(SEARCH_DEBOUNCE_MS);
    expect(seen).toEqual(['b']);
  });

  it('is safe to cancel when nothing is waiting', () => {
    const call = build<string>(() => undefined);

    expect(() => {
      call.cancel();
      call.cancel();
    }).not.toThrow();
  });

  /**
   * Why it must be built in an injection context: a pending call belonging to a
   * component that has gone would act on state nobody is looking at — and in a spec, it
   * fires after the test that created it.
   */
  it('drops a pending call when its injection context is destroyed', () => {
    const seen: string[] = [];
    const call = build<string>((value) => seen.push(value));

    call('a');
    destroy();
    vi.advanceTimersByTime(SEARCH_DEBOUNCE_MS);

    expect(seen).toEqual([]);
  });

  it('refuses to be built outside an injection context', () => {
    expect(() => debounced(() => undefined, SEARCH_DEBOUNCE_MS)).toThrow();
  });

  it('exposes the delay the search actually uses', () => {
    expect(SEARCH_DEBOUNCE_MS).toBe(150);
    expect(TestBed.inject(DestroyRef)).toBeDefined();
  });
});
