import type { ExportReport, ExportScope, ImportReport, SearchHit } from '@core/ipc/bindings';
import { LanguageTag } from '@core/model/language.model';
import { ChecklistItem, NoteKind } from './checklist.model';
import type { NoteFolder } from './folder.model';

export { type ChecklistItem, type NoteKind } from './checklist.model';

export type { SearchField, SearchHit } from '@core/ipc/bindings';

/** Both cross as themselves: a batch answers what it changed, and the undo hands it back. */
export type { NotePlacement, NoteTag } from '@core/ipc/bindings';

export type NoteLifecycle = { readonly kind: 'permanent' } | { readonly kind: 'expires'; readonly at: Date };

/** Two variants carry a date and not a label, so the text ages without a round trip. */
export type NoteFooter =
  | { readonly kind: 'source'; readonly value: string }
  | { readonly kind: 'expiry'; readonly at: Date }
  | { readonly kind: 'age'; readonly at: Date };

/** Everything `NoteDraft` omits below is derived by the back end and never written. */
export interface Note {
  readonly id: string;
  readonly spaceId: string;
  /** `null` = unfiled, which is a legitimate state and the one a card says nothing about. */
  readonly folderId: string | null;
  /** Resolved by the back end: the card never joins `folderId` against a list it holds. */
  readonly folder: NoteFolder | null;
  readonly title: string;
  readonly language: LanguageTag;
  /** ⚠️ Only the first lines when `truncated`: a list sends previews. */
  readonly content: string;
  /** Context, e.g. "API Gateway / Auth" — its first segment is the label. */
  readonly source: string;
  readonly tags: readonly string[];
  readonly pinned: boolean;
  readonly createdAt: Date;
  readonly updatedAt: Date;
  readonly lifecycle: NoteLifecycle;
  readonly footer: NoteFooter;
  readonly expiringSoon: boolean;
  readonly placeholders: readonly Placeholder[];
  readonly attachmentCount: number;
  readonly kind: NoteKind;
  /** A todo list has these instead of `content`. */
  readonly items: readonly ChecklistItem[];
  /** What copying yields when that is not `content`; `null` for a snippet. */
  readonly copyText: string | null;
  /** Why this note is in the results; `null` outside a search and for a title match. */
  readonly searchHit: SearchHit | null;
  /** `content` was cut to a preview; `NotesRepository.whole` reads the rest. */
  readonly truncated: boolean;
}

/**
 * One seeded note, and which of the seeded folders it lands in. ⚠️ An **index**, not an
 * id: the folders do not exist until the command that writes them runs.
 */
export interface SampleNote {
  readonly folder: number | undefined;
  readonly draft: NoteDraft;
}

export type NoteDraft = Omit<
  Note,
  | 'id'
  | 'folder'
  | 'createdAt'
  | 'updatedAt'
  | 'footer'
  | 'expiringSoon'
  | 'placeholders'
  | 'attachmentCount'
  | 'copyText'
  | 'searchHit'
  | 'truncated'
>;

/**
 * ⚠️ No `folderId`: filing has a command of its own (`FoldersRepository.fileMany`), so
 * a patch can never refile a note as a side effect. The back end has no field for it
 * either — the only move it makes is unfiling a note that changes space.
 */
export type NotePatch = Partial<Omit<NoteDraft, 'folderId'>>;

/** `untriaged` = notes carrying a deadline, the ones whose fate is undecided. */
export type NoteFilter = 'all' | 'pinned' | 'untriaged';

export interface NotesQuery {
  /** `null` = "all spaces", a choice and not an absence of one. */
  readonly spaceId: string | null;
  /** `null` = every folder, filed or not. */
  readonly folderId: string | null;
  readonly search: string;
  readonly filter: NoteFilter;
  /** Union semantics, like `languages`: at least one of them. Empty = all. */
  readonly tags: readonly string[];
  readonly languages: readonly LanguageTag[];
  readonly now: Date;
  /**
   * ⚠️ `Date#getTimezoneOffset()`. The sections reason in local days: without this
   * offset a note created at 11 pm lands in the wrong one.
   */
  readonly tzOffsetMinutes: number;
  /** Their own section when the view is chronological, the head of the list when flat. */
  readonly pinnedFirst: boolean;
}

/** No flat list: one would invite re-filtering what the back end has already done. */
export interface NotesView {
  readonly sections: readonly NoteSection[];
  readonly availableTags: readonly string[];
  readonly availableLanguages: readonly LanguageTag[];
  readonly isFiltering: boolean;
  readonly matched: number;
}

/** Doubles as a translation key (`'sections.' + key`). */
export type NoteSectionKey = 'pinned' | 'today' | 'week' | 'older' | 'results';

export interface NoteSection {
  readonly key: NoteSectionKey;
  readonly notes: readonly Note[];
  readonly hasExpiringNotes: boolean;
  readonly showCreateGhost: boolean;
}

/**
 * The fields come from the text and the values from the database; the back end pairs
 * them here, so a value whose token has left the content simply stops being a field.
 */
export interface Placeholder {
  readonly name: string;
  readonly defaultValue: string;
  readonly value: string;
}

/** Not a `Note`: a discarded note is restored or purged, never opened, so nothing is decorated. */
export interface TrashedNote {
  readonly id: string;
  readonly spaceId: string;
  readonly title: string;
  readonly language: LanguageTag;
  readonly content: string;
  readonly tags: readonly string[];
  readonly deletedAt: Date;
  readonly purgeAt: Date;
  readonly kind: NoteKind;
}

export interface TagUsage {
  readonly tag: string;
  readonly noteCount: number;
}

/** The bytes are not here: they arrive on demand, as a `data:` URI. */
export interface Attachment {
  readonly id: string;
  readonly noteId: string;
  /** The original name, displayed as is. ⚠️ Never used as a path. */
  readonly fileName: string;
  readonly mimeType: string;
  readonly byteSize: number;
  readonly createdAt: Date;
}

export type { ExportReport, ExportScope, ImportReport };
