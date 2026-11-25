import { Injectable, signal } from '@angular/core';

/** A folder's colour, as the tokens name its RGB triplet (`--green-rgb`). */
export type Tone = 'green' | 'blue' | 'purple' | 'teal' | 'olive' | 'pink' | 'grey' | 'amber';

export interface ProgressSegment {
  readonly anchor: string;
  readonly name: string;
  readonly tone: Tone;
}

export interface ProgressReading {
  /** Each segment's length, as long as its section. */
  readonly weights: readonly number[];
  /** How much of each section has been read, 0…1. */
  readonly fills: readonly number[];
  /** The section the middle of the screen is in, or -1 above the first. */
  readonly current: number;
}

const AT_REST: ProgressReading = { weights: [], fills: [], current: -1 };

const same = (a: ProgressReading, b: ProgressReading) =>
  a.current === b.current &&
  a.weights.length === b.weights.length &&
  a.weights.every((weight, i) => weight === b.weights[i] && a.fills[i] === b.fills[i]);

/**
 * The line under the header and the name beside the logo: what a long page has shown of
 * itself. A page declares its sections and reports its scroll; the header draws them.
 */
@Injectable({ providedIn: 'root' })
export class PageProgress {
  private readonly _segments = signal<readonly ProgressSegment[]>([]);
  private readonly _reading = signal<ProgressReading>(AT_REST, { equal: same });
  readonly segments = this._segments.asReadonly();
  readonly reading = this._reading.asReadonly();

  show(segments: readonly ProgressSegment[]): void {
    this._segments.set(segments);
    this._reading.set({ weights: segments.map(() => 1), fills: segments.map(() => 0), current: -1 });
  }

  report(reading: ProgressReading): void {
    this._reading.set(reading);
  }

  clear(): void {
    this._segments.set([]);
    this._reading.set(AT_REST);
  }
}
