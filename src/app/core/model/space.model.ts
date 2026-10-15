export type { SpaceContents } from '@core/ipc/bindings';

export interface Space {
  readonly id: string;
  readonly name: string;
  /** Hoisted to the head of the list, the way a pinned note is on the canvas. */
  readonly pinned: boolean;
}

export type SpaceDraft = Omit<Space, 'id' | 'pinned'>;
