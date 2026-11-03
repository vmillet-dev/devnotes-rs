/**
 * ⚠️ Node ≥ 22 ships a `localStorage` global that shadows jsdom's and evaluates to
 * `undefined` without `--localstorage-file`. The replacement is installed
 * unconditionally: finding out whether the global works means reading the accessor,
 * which is exactly what prints the `ExperimentalWarning`.
 */

function createStorage(): Storage {
  const entries = new Map<string, string>();

  return {
    get length(): number {
      return entries.size;
    },
    key(index: number): string | null {
      return [...entries.keys()][index] ?? null;
    },
    getItem(key: string): string | null {
      return entries.get(String(key)) ?? null;
    },
    // The real thing stringifies both sides, like the WebView would.
    setItem(key: string, value: string): void {
      entries.set(String(key), String(value));
    },
    removeItem(key: string): void {
      entries.delete(String(key));
    },
    clear(): void {
      entries.clear();
    },
  };
}

Object.defineProperty(globalThis, 'localStorage', {
  value: createStorage(),
  configurable: true,
  enumerable: false,
  writable: false,
});
