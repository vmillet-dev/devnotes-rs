import type {
  BoardFrame,
  BoardNote as WireBoardNote,
  BoardView as WireBoardView,
  BoardZone as WireBoardZone,
} from '@core/ipc/bindings';
import { Folder } from './folder.model';
import { Note, NoteFilter } from './note.model';
import { LanguageTag } from './language.model';

export type {
  BoardArrangement,
  BoardFrame,
  BoardLayout,
  BoardPoint,
  BoardScope,
  CardPlacement,
  ZonePlacement,
} from '@core/ipc/bindings';

/** Kept out of `Note`: where a card sits is local, and must never travel with an export. */
export interface BoardNote {
  readonly note: Note;
  /** ⚠️ Dimmed in place, never dropped — a reflow throws the spatial memory away. */
  readonly matches: boolean;
  /** `null` inside a zone, where a card flows; set only on the free background. */
  readonly position: WireBoardNote['position'];
}

export interface BoardZone {
  readonly folder: Folder;
  readonly frame: BoardFrame;
  readonly notes: readonly BoardNote[];
}

export interface BoardView {
  readonly zones: readonly BoardZone[];
  readonly loose: readonly BoardNote[];
  readonly availableTags: readonly string[];
  readonly availableLanguages: readonly LanguageTag[];
  readonly isFiltering: boolean;
  readonly matched: number;
  /** The surface to pan over, sized by the back end from what is actually on it. */
  readonly width: number;
  readonly height: number;
}

/** ⚠️ `spaceId` is required: a folder belongs to a space, so a board across all of them
 *  would have no zones to draw. */
export interface BoardQuery {
  readonly spaceId: string;
  readonly search: string;
  readonly filter: NoteFilter;
  readonly tags: readonly string[];
  readonly languages: readonly LanguageTag[];
  readonly now: Date;
}

/** Per space, so arranging one does not switch the others. */
export type NotesViewMode = 'date' | 'board';

export type { WireBoardNote, WireBoardView, WireBoardZone };
