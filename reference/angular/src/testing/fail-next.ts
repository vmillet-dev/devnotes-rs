/** A spec sets `failNext` to make the next call reject once. */
export interface FailsNext {
  failNext: Error | null;
}

export function guard<T>(owner: FailsNext, operation: () => T): Promise<T> {
  if (owner.failNext) {
    const error = owner.failNext;
    owner.failNext = null;
    return Promise.reject(error);
  }
  try {
    return Promise.resolve(operation());
  } catch (error) {
    return Promise.reject(error);
  }
}
