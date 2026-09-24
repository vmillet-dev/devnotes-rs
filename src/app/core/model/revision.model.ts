/** A line of the preview: what going back to a version would keep, bring back or take away. */
export type { DiffLine } from '@core/ipc/bindings';

/** A body kept beside a note, before an edit replaced it: metadata only. */
export interface Revision {
  readonly id: string;
  readonly takenAt: Date;
  /** How much the version held, so a row can say it without carrying it. */
  readonly characters: number;
}
