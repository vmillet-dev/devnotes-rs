/**
 * At most one call in flight; a caller arriving meanwhile waits for it, and gives up if its
 * signal aborts first. Paired with `resource`, which aborts a loader its params outran, a
 * burst of queries sends the one in flight and the newest, never the ones in between: every
 * command queues behind one lock in Rust, which would compute each of them to the end.
 */
export class OneInFlight {
  private current: Promise<unknown> | null = null;

  async run<T>(signal: AbortSignal, call: () => Promise<T>): Promise<T> {
    while (this.current !== null) {
      await this.current.catch(() => undefined);
      signal.throwIfAborted();
    }

    const running = call();
    this.current = running;
    try {
      return await running;
    } finally {
      if (this.current === running) this.current = null;
    }
  }
}
