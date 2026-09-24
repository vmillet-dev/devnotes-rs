import type { Folder as WireFolder } from '@core/ipc/bindings';

/** Generated, so a colour added in Rust is a compile error here rather than a blank swatch. */
export type { FolderColour, NoteFolder, NoteFiling } from '@core/ipc/bindings';

/**
 * `colour` carries `#[serde(default)]` on the Rust side so an export file written
 * before folders stays readable, and specta turns that into an optional key — which the
 * model refuses to be. Same seam as `Space.pinned`.
 */
export interface Folder {
  readonly id: string;
  readonly spaceId: string;
  readonly name: string;
  readonly colour: NonNullable<WireFolder['colour']>;
  readonly createdAt: Date;
}

export interface FolderDraft {
  readonly spaceId: string;
  readonly name: string;
}
