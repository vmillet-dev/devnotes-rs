import { describe, expect, it } from 'vitest';
import { Unlisten, subscribeCancellable } from './subscription.util';

/** A subscription that only lands when the spec says so — which is the whole subject. */
function deferred(): {
  subscribe: (handler: () => void) => Promise<Unlisten>;
  land: () => Promise<void>;
  fail: (error: Error) => Promise<void>;
  stopped: () => number;
  handler: () => (() => void) | null;
} {
  let settle: (() => void) | null = null;
  let reject: ((error: Error) => void) | null = null;
  let stopped = 0;
  let held: (() => void) | null = null;

  return {
    subscribe: (handler) =>
      new Promise<Unlisten>((resolve, rejectWith) => {
        settle = () => {
          held = handler;
          resolve(() => {
            stopped += 1;
            held = null;
          });
        };
        reject = rejectWith;
      }),
    land: async () => {
      settle?.();
      await Promise.resolve();
    },
    fail: async (error) => {
      reject?.(error);
      await Promise.resolve();
    },
    stopped: () => stopped,
    handler: () => held,
  };
}

describe('subscribeCancellable', () => {
  it('passes the handler on, and hands back a way to stop', async () => {
    const source = deferred();

    const stop = subscribeCancellable(source.subscribe, () => undefined);
    await source.land();

    expect(source.handler()).not.toBeNull();
    stop();
    expect(source.stopped()).toBe(1);
  });

  /**
   * The reason this helper exists: the subscription only lands on the next turn, and a
   * caller destroyed before then would stay subscribed for the rest of the session — with
   * nothing left holding the unlisten to call it.
   */
  it('stops a subscription that had not landed when it was cancelled', async () => {
    const source = deferred();

    const stop = subscribeCancellable(source.subscribe, () => undefined);
    stop();
    await source.land();

    expect(source.stopped()).toBe(1);
    expect(source.handler()).toBeNull();
  });

  it('stops once, however many times it is asked', async () => {
    const source = deferred();

    const stop = subscribeCancellable(source.subscribe, () => undefined);
    await source.land();

    stop();
    stop();
    stop();

    expect(source.stopped()).toBe(1);
  });

  /** A subscription that never arrives is not a crash: the feature is simply absent. */
  it('swallows a subscription that failed to open', async () => {
    const source = deferred();

    const stop = subscribeCancellable(source.subscribe, () => undefined);
    await source.fail(new Error('no bridge'));

    expect(() => stop()).not.toThrow();
    expect(source.stopped()).toBe(0);
  });
});
