import { signal } from '@angular/core';

/** How long a move stays marked: as long as its fade, `--motion-mark`. */
export const MARK_MS = 2000;

/** A move on screen: the row, and what now holds it. */
export interface Moved {
  readonly id: string;
  readonly parentId: string | null;
}

/** What the live region says of a move, made or refused. */
export interface Said {
  /** `into`, `refused.last`…: the sentence without its words, for what reads it in no language. */
  readonly sentence: string;
  readonly key: string;
  readonly params: Readonly<Record<string, string>>;
  /** Told apart from the one before: the same sentence twice is still two things said. */
  readonly beat: number;
}

/**
 * Where a move landed, shown and said. The pointer shows where a drop would land; the keys
 * showed nothing, and a row above a folder looks like a row inside it. Each move is marked, on
 * the row and on what holds it, and said in the rail's live region.
 */
export class RailMarks {
  private timer: ReturnType<typeof setTimeout> | undefined;

  private readonly _moved = signal<Moved | null>(null);
  private readonly _beat = signal(0);
  private readonly _said = signal<readonly Said[]>([]);

  readonly moved = this._moved.asReadonly();
  readonly beat = this._beat.asReadonly();
  readonly said = this._said.asReadonly();

  /** `prefix` is the translation path the sentences sit under, `http.rail.moved`… */
  constructor(private readonly prefix: string) {}

  /** ⚠️ The beat changes with every move: it is what starts the fade again on a row already marked. */
  mark(moved: Moved): void {
    clearTimeout(this.timer);
    this._moved.set(moved);
    this._beat.update((beat) => beat + 1);
    this.timer = setTimeout(() => this._moved.set(null), MARK_MS);
  }

  say(sentence: string, params: Readonly<Record<string, string>>): void {
    this._said.update(([last]) => [
      { sentence, key: `${this.prefix}.${sentence}`, params, beat: (last?.beat ?? 0) + 1 },
    ]);
  }

  markOf(id: string): 'row' | 'parent' | null {
    const moved = this._moved();
    if (moved === null) return null;
    return moved.id === id ? 'row' : moved.parentId === id ? 'parent' : null;
  }

  dispose(): void {
    clearTimeout(this.timer);
  }
}
