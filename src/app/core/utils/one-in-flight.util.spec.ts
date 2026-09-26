import { describe, expect, it } from 'vitest';
import { OneInFlight } from './one-in-flight.util';

/** A call the spec settles by hand, and a record of when it was sent. */
function deferred(sent: string[], name: string): { call: () => Promise<string>; settle: () => void } {
  let settle: () => void = () => undefined;
  return {
    call: () => {
      sent.push(name);
      return new Promise<string>((resolve) => (settle = () => resolve(name)));
    },
    settle: () => settle(),
  };
}

describe('OneInFlight', () => {
  it('sends the next call only once the one in flight has answered', async () => {
    const gate = new OneInFlight();
    const sent: string[] = [];
    const first = deferred(sent, 'first');
    const second = deferred(sent, 'second');

    const firstAnswer = gate.run(new AbortController().signal, first.call);
    const secondAnswer = gate.run(new AbortController().signal, second.call);
    await Promise.resolve();
    expect(sent).toEqual(['first']);

    first.settle();
    await expect(firstAnswer).resolves.toBe('first');
    await Promise.resolve();
    expect(sent).toEqual(['first', 'second']);

    second.settle();
    await expect(secondAnswer).resolves.toBe('second');
  });

  /** What a burst of keystrokes sends: the query running, then the newest. */
  it('never sends a call whose caller gave up while it waited', async () => {
    const gate = new OneInFlight();
    const sent: string[] = [];
    const running = deferred(sent, 'running');
    const outrun = deferred(sent, 'outrun');
    const newest = deferred(sent, 'newest');
    const abandoned = new AbortController();

    void gate.run(new AbortController().signal, running.call);
    const outrunAnswer = gate.run(abandoned.signal, outrun.call);
    const newestAnswer = gate.run(new AbortController().signal, newest.call);
    abandoned.abort();

    running.settle();
    await expect(outrunAnswer).rejects.toThrow();
    await Promise.resolve();
    expect(sent).toEqual(['running', 'newest']);

    newest.settle();
    await expect(newestAnswer).resolves.toBe('newest');
  });

  it('lets the next call through after one that failed', async () => {
    const gate = new OneInFlight();

    await expect(
      gate.run(new AbortController().signal, () => Promise.reject(new Error('refused'))),
    ).rejects.toThrow('refused');

    await expect(gate.run(new AbortController().signal, () => Promise.resolve('answered'))).resolves.toBe(
      'answered',
    );
  });
});
