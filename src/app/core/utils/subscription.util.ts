export type Unlisten = () => void;

/**
 * Makes a subscription that only lands on the next turn cancellable now: without the
 * flag, a caller destroyed before the promise resolves stays subscribed for the session.
 */
export function subscribeCancellable<T>(subscribe: (handler: T) => Promise<Unlisten>, handler: T): Unlisten {
  let unlisten: Unlisten | null = null;
  let cancelled = false;

  void subscribe(handler)
    .then((stop) => {
      if (cancelled) {
        stop();
        return;
      }
      unlisten = stop;
    })
    .catch(() => undefined);

  return () => {
    cancelled = true;
    unlisten?.();
    unlisten = null;
  };
}
