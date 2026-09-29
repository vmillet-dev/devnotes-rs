import { ApplicationRef, Injector, runInInjectionContext, signal } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { provideAppTesting } from '@testing/testing.providers';
import { LIVE_RESULT_DEBOUNCE_MS, LiveResult, liveResult } from './live-result';

describe('liveResult', () => {
  let text: ReturnType<typeof signal<string>>;
  let calls: string[];
  let fail: boolean;
  let live: LiveResult<string>;

  async function settle(): Promise<void> {
    await vi.advanceTimersByTimeAsync(LIVE_RESULT_DEBOUNCE_MS);
    await TestBed.inject(ApplicationRef).whenStable();
  }

  beforeEach(() => {
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({ providers: [provideAppTesting()] });
    text = signal('Été');
    calls = [];
    fail = false;
    live = runInInjectionContext(TestBed.inject(Injector), () =>
      liveResult(
        () => (text() === '' ? undefined : { text: text() }),
        async ({ text: asked }) => {
          calls.push(asked);
          if (fail) throw new Error('bridge down');
          return asked.toUpperCase();
        },
      ),
    );
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('asks at once for what a tool was found holding', async () => {
    TestBed.tick();
    await settle();

    expect(calls).toEqual(['Été']);
    expect(live.value()).toBe('ÉTÉ');
  });

  it('waits for the typing to pause, and asks once for the same request', async () => {
    TestBed.tick();
    await settle();

    text.set('Été 2');
    TestBed.tick();
    text.set('Été 20');
    TestBed.tick();
    expect(live.pending()).toBe(true);
    await settle();

    expect(calls).toEqual(['Été', 'Été 20']);
    expect(live.value()).toBe('ÉTÉ 20');
    expect(live.pending()).toBe(false);
  });

  it('empties when nothing is asked', async () => {
    TestBed.tick();
    await settle();

    text.set('');
    TestBed.tick();
    await settle();

    expect(live.value()).toBeNull();
  });

  it('reports the unexpected, and keeps the last answer on screen', async () => {
    TestBed.tick();
    await settle();
    // Read as a template reads it: a `linkedSignal` keeps only what it has been read for.
    expect(live.value()).toBe('ÉTÉ');

    fail = true;
    text.set('x');
    TestBed.tick();
    await settle();

    expect(live.value()).toBe('ÉTÉ');
    expect(TestBed.inject(ErrorNotifier).notice()?.ref.key).toBe('errors.toolFailed');
  });
});
