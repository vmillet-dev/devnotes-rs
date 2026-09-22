import { Injectable, Signal, computed, inject, signal } from '@angular/core';
import { FoldersRepository } from '../data/folders.repository';
import { NotesRepository } from '../data/notes.repository';
import { ClipboardService } from '@core/services/clipboard/clipboard.service';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { FALLBACK_LANGUAGE } from '@core/model/language.model';
import {
  ChecklistItem,
  Note,
  NoteDraft,
  NoteKind,
  NoteLifecycle,
  NotePatch,
  NotePlacement,
  NoteTag,
} from '../model/note.model';
import { NoteFiling } from '../model/folder.model';
import { BoardLayout, BoardScope } from '../model/board.model';
import { ClockService } from '@core/services/time/clock.service';
import { debounced } from '@core/services/time/debounce';
import { NoteSelectionStore } from './note-selection.store';
import { FoldersStore } from './folders.store';
import { BoardStore } from './board.store';
import { NotesQueryStore } from './notes-query.store';
import { NotesRevision } from './notes-revision';
import { SpacesStore } from './spaces.store';

export type { NoteFilter, NoteKind } from '../model/note.model';

/** The note is not lost after that: it stays in the trash for 30 days. */
export const UNDO_WINDOW_MS = 8000;

/**
 * ⚠️ Each variant carries what the back end answered, never what the front end guessed.
 * Rebuilding the pairs from the selection would undo a tag the note already carried.
 */
export type Reversible =
  | { readonly kind: 'deletion'; readonly ids: readonly string[]; readonly count: number }
  | { readonly kind: 'move'; readonly previous: readonly NotePlacement[]; readonly count: number }
  | { readonly kind: 'tag'; readonly added: readonly NoteTag[]; readonly count: number }
  | { readonly kind: 'file'; readonly previous: readonly NoteFiling[]; readonly count: number }
  | { readonly kind: 'arrange'; readonly layout: BoardLayout; readonly count: number };

/** The note being created, not written until it is worth keeping. */
export const DRAFT_ID = '__draft__';

function isWorthSaving(note: Note): boolean {
  return (
    note.title.trim() !== '' ||
    note.content.trim() !== '' ||
    note.source.trim() !== '' ||
    note.tags.length > 0 ||
    note.items.length > 0 ||
    note.pinned ||
    note.lifecycle.kind === 'expires'
  );
}

/** Empty title: the UI renders a translated placeholder, and storing one would freeze a language into the data. */
function emptyDraft(spaceId: string, kind: NoteKind, folderId: string | null = null): NoteDraft {
  return {
    spaceId,
    folderId,
    title: '',
    language: FALLBACK_LANGUAGE,
    content: '',
    source: '',
    tags: [],
    pinned: false,
    lifecycle: { kind: 'permanent' },
    kind,
    items: [],
  };
}

function emptyNote(spaceId: string, now: Date, kind: NoteKind, folderId: string | null = null): Note {
  return {
    ...emptyDraft(spaceId, kind, folderId),
    id: DRAFT_ID,
    createdAt: now,
    updatedAt: now,
    footer: { kind: 'age', at: now },
    expiringSoon: false,
    placeholders: [],
    attachmentCount: 0,
    folder: null,
    copyText: null,
    searchHit: null,
  };
}

function toDraftPayload(note: Note): NoteDraft {
  return {
    spaceId: note.spaceId,
    folderId: note.folderId,
    title: note.title,
    language: note.language,
    content: note.content,
    source: note.source,
    tags: [...note.tags],
    pinned: note.pinned,
    lifecycle: note.lifecycle,
    kind: note.kind,
    items: note.items.map((item) => ({ ...item })),
  };
}

function sameStrings(a: readonly string[], b: readonly string[]): boolean {
  return a.length === b.length && a.every((value, index) => value === b[index]);
}

function sameItems(a: readonly ChecklistItem[], b: readonly ChecklistItem[]): boolean {
  return (
    a.length === b.length &&
    a.every((item, index) => item.text === b[index]?.text && item.done === b[index]?.done)
  );
}

/** By value: `Date` compares by identity, so the same deadline would read as a change. */
function sameLifecycle(a: NoteLifecycle, b: NoteLifecycle): boolean {
  if (a.kind !== b.kind) return false;
  return a.kind !== 'expires' || a.at.getTime() === (b as { at: Date }).at.getTime();
}

/** Exhaustive by construction: a new `NotePatch` field stops this compiling. */
const UNCHANGED: {
  readonly [K in keyof Required<NotePatch>]: (current: Note[K], next: Required<NotePatch>[K]) => boolean;
} = {
  spaceId: Object.is,
  title: Object.is,
  language: Object.is,
  content: Object.is,
  source: Object.is,
  pinned: Object.is,
  kind: Object.is,
  tags: sameStrings,
  items: sameItems,
  lifecycle: sameLifecycle,
};

function changedFields(note: Note, patch: NotePatch): NotePatch {
  const changed: Record<string, unknown> = {};

  for (const key of Object.keys(patch) as (keyof NotePatch)[]) {
    const next = patch[key];
    // An absent key is "do not touch", which is what serde reads on the far side.
    if (next === undefined) continue;

    const unchanged = UNCHANGED[key] as (current: unknown, next: unknown) => boolean;
    if (!unchanged(note[key], next)) {
      changed[key] = next;
    }
  }

  return changed;
}

@Injectable({ providedIn: 'root' })
export class NotesStore {
  private readonly repository = inject(NotesRepository);
  private readonly folders = inject(FoldersRepository);
  private readonly openFolder = inject(FoldersStore);
  private readonly clipboard = inject(ClipboardService);
  private readonly clock = inject(ClockService);
  private readonly notifier = inject(ErrorNotifier);
  private readonly spaces = inject(SpacesStore);
  private readonly notes = inject(NotesQueryStore);
  private readonly board = inject(BoardStore);
  private readonly revision = inject(NotesRevision);
  private readonly selection = inject(NoteSelectionStore);

  private readonly _selectedNote = signal<Note | null>(null);
  private readonly _draftNote = signal<Note | null>(null);

  /**
   * What the editor's local drafts key on. ⚠️ Bumped when the editor is pointed at a
   * different note, and deliberately not when a draft adopts the row it became: that
   * changes the id of the *same* note, and stale drafts would be replayed over it.
   */
  private readonly _editorSession = signal(0);
  private readonly _lastAction = signal<Reversible | null>(null);
  private readonly _undoVisible = signal(false);

  /** The draft wins: while it exists, it is what the editor shows. */
  readonly selectedNote = computed<Note | null>(() => this._draftNote() ?? this._selectedNote());
  readonly selectedNoteId = computed<string | null>(() => this.selectedNote()?.id ?? null);

  readonly editorSession: Signal<number> = this._editorSession.asReadonly();

  /** The id actually in the database, or `null` while the open note is only a draft. */
  readonly persistedNoteId = computed<string | null>(() => this._selectedNote()?.id ?? null);
  readonly lastAction = this._lastAction.asReadonly();

  readonly undoBanner = computed<Reversible | null>(() => (this._undoVisible() ? this._lastAction() : null));

  private readonly hideUndoBanner = debounced(() => this._undoVisible.set(false), UNDO_WINDOW_MS);

  /**
   * ⚠️ The **promise**, not the id it will yield. `requestClose()` fires three commits
   * back to back with no `await` between them; holding the id — which exists only once
   * the write returns — leaves that window answering "still a draft", and one close
   * then creates two notes.
   */
  private draftMaterialisation: Promise<string | null> | null = null;

  /**
   * ⚠️ The row a draft became, held until the editor moves on. Commits are still in
   * flight when the overlay closes, and the canvas view is a round trip behind the
   * write that created the note — without this they resolve against nothing and the
   * body typed after the title is dropped.
   */
  private materialisedNote: Note | null = null;

  /**
   * ⚠️ A `Note` and not only an id, for the palette. It queries every space and ignores
   * the canvas filters — deliberately — so `find`, whose last rung is the canvas view and
   * the board view, has nothing that can resolve one of its results: a note the filters
   * hide used to open onto `null`, which is the editor not opening at all and nothing on
   * screen saying why (#280). The row is handed over rather than read again; a
   * `find_note` on the bridge would be a second source for what the caller already holds.
   */
  openNote(wanted: Note | string): void {
    const note = typeof wanted === 'string' ? this.find(wanted) : wanted;

    this.materialisedNote = null;
    this.discardDraft();
    this._editorSession.update((session) => session + 1);
    this._selectedNote.set(note);
    // Nothing resolved is nothing to point at: the ring stays where the user left it.
    if (note) {
      this.selection.focusNote(note.id);
    }
  }

  /** A draft still empty on close is abandoned, not saved. */
  closeOverlay(): void {
    this.discardDraft();
    this._editorSession.update((session) => session + 1);
    this._selectedNote.set(null);
  }

  /** The single write for a note's own fields; what has not moved is dropped. */
  applyPatch(id: string, patch: NotePatch): Promise<void> {
    return this.edit(id, (note) => {
      const changes = changedFields(note, patch);
      return Object.keys(changes).length === 0 ? null : changes;
    });
  }

  togglePinned(id: string): Promise<void> {
    return this.edit(id, (note) => ({ pinned: !note.pinned }));
  }

  moveNote(id: string, spaceId: string): Promise<void> {
    return this.applyPatch(id, { spaceId });
  }

  /**
   * Files one note, which is a **batch of one**.
   *
   * ⚠️ Not a `NotePatch` field, and deliberately not a command of its own either: filing
   * goes through `file_notes`, which answers the placements it actually changed — that
   * answer is what the undo puts back, and a second path would drift from the selection
   * bar's. A draft is materialised first: a note with no row cannot be filed.
   */
  async fileNote(id: string, folderId: string | null): Promise<void> {
    const resolved = await this.resolve(id);
    const target = resolved === DRAFT_ID ? await this.materialiseDraft() : resolved;
    if (!target) return;

    const previous = await this.notifier.attempt('errors.fileFailed', () =>
      this.folders.fileMany([target], folderId),
    );
    if (previous === null) return;

    this.adoptFiling(target, folderId);
    this.revision.bump();
    this.openUndoWindow({ kind: 'file', previous, count: previous.length });
  }

  /**
   * ⚠️ The one write that has no note to adopt. `file_notes` answers the placements it
   * changed, not the rows — so the open note kept the folder it had, and the editor's own
   * control went on naming it until the note was closed and reopened.
   *
   * It is not a guess: the filing is exactly what was asked for and accepted, and the
   * folder is resolved from the same list the control offered. The canvas still reloads,
   * which is what refreshes the card's chip.
   */
  private adoptFiling(id: string, folderId: string | null): void {
    const open = this._selectedNote();
    if (open?.id !== id) return;

    const folder = this.openFolder.allFolders().find((each) => each.id === folderId);
    this._selectedNote.set({
      ...open,
      folderId,
      folder: folder ? { id: folder.id, name: folder.name, colour: folder.colour } : null,
    });
  }

  /** Replaces the whole list: an item has no identity beyond its position. */
  setChecklist(id: string, items: readonly ChecklistItem[]): Promise<void> {
    return this.applyPatch(id, { items });
  }

  /** Opens the editor on a local draft; a note with no space at all is refused. */
  /**
   * ⚠️ A note made while a folder is open arrives already filed. That and a drop on the
   * board are the only two places that file a new one: the quick-paste palette
   * deliberately does not, because it is used mid-task from another application and a
   * decision there would sit in the fastest path in the product.
   */
  createNote(kind: NoteKind = 'snippet'): void {
    const spaceId = this.spaceForNewNote();
    if (!spaceId) return;

    this.materialisedNote = null;
    this._selectedNote.set(null);
    this.draftMaterialisation = null;
    this._editorSession.update((session) => session + 1);
    this._draftNote.set(emptyNote(spaceId, this.clock.now(), kind, this.openFolder.activeFolderId()));
  }

  async captureFromClipboard(): Promise<void> {
    await this.createWithContent(await this.clipboard.paste());
  }

  /** Already carries its content, so no draft: the editor opens on a saved note. */
  async createWithContent(content: string): Promise<void> {
    if (!content.trim()) return;

    const spaceId = this.spaceForNewNote();
    if (!spaceId) return;

    await this.persistNew({ ...emptyDraft(spaceId, 'snippet'), content });
  }

  async materialiseDraft(): Promise<string | null> {
    const persisted = this.persistedNoteId();
    if (persisted) return persisted;

    const draft = this._draftNote();
    if (!draft) return null;

    return this.saveDraft(draft);
  }

  async deleteNote(id: string): Promise<void> {
    const resolved = await this.resolve(id);

    // A draft exists nowhere: nothing to trash, so nothing to undo either.
    if (resolved === DRAFT_ID) {
      this.closeOverlay();
      return;
    }

    if (!this.find(resolved)) return;

    const deleted = await this.notifier.attempt('errors.noteDeleteFailed', () =>
      this.repository.delete(resolved),
    );
    if (deleted === null) return;

    if (this.selectedNoteId() === resolved) {
      this.closeOverlay();
    }
    this.openUndoWindow({ kind: 'deletion', ids: [resolved], count: 1 });
    this.revision.bump();
  }

  async moveSelection(spaceId: string): Promise<void> {
    const previous = await this.runOnSelection((ids) => this.repository.moveMany(ids, spaceId));
    if (previous === null) return;

    this.openUndoWindow({ kind: 'move', previous, count: previous.length });
  }

  /** `folderId` of `null` takes the selection out of whatever folder it was in. */
  async fileSelection(folderId: string | null): Promise<void> {
    const previous = await this.runOnSelection((ids) => this.folders.fileMany(ids, folderId));
    if (previous === null) return;

    this.openUndoWindow({ kind: 'file', previous, count: previous.length });
  }

  async tagSelection(tag: string): Promise<void> {
    if (!tag.trim()) return;

    // No normalisation here: `notes::model::normalize_tags` is its only keeper.
    const added = await this.runOnSelection((ids) => this.repository.tagMany(ids, [tag]));
    if (added === null) return;

    this.openUndoWindow({ kind: 'tag', added, count: added.length });
  }

  async deleteSelection(): Promise<void> {
    const ids = this.selection.checkedNoteIds();
    if (ids.length === 0) return;

    const count = await this.notifier.attempt('errors.bulkActionFailed', () =>
      this.repository.deleteMany(ids),
    );
    if (count === null) return;

    this.selection.clearSelection();
    this.openUndoWindow({ kind: 'deletion', ids, count });
    this.revision.bump();
  }

  async undoLastAction(): Promise<void> {
    const action = this._lastAction();
    if (!action) return;

    this.dismissUndo();
    const undone = await this.notifier.attempt('errors.undoFailed', () => this.reverse(action));
    if (undone !== null) this.revision.bump();
  }

  /** Hiding the banner gives up the undo, unlike the timer running out. */
  dismissUndo(): void {
    this.hideUndoBanner.cancel();
    this._undoVisible.set(false);
    this._lastAction.set(null);
  }

  /** Exhaustive by construction: a new kind of undo stops this compiling. */
  private reverse(action: Reversible): Promise<number> {
    switch (action.kind) {
      case 'deletion':
        return this.repository.restore(action.ids);
      case 'move':
        return this.repository.moveBack(action.previous);
      case 'tag':
        return this.repository.untagMany(action.added);
      case 'file':
        return this.folders.fileBack(action.previous);
      case 'arrange':
        return this.board.restoreLayout(action.layout);
    }
  }

  /**
   * Puts the board back in order, and offers the previous arrangement back.
   *
   * ⚠️ Here rather than on `BoardStore`, which cannot reach the undo: this store injects
   * the board, so the dependency only runs one way.
   *
   * ⚠️ The count is what actually **moved**, which the back end works out: a board already
   * in order opens no undo window, because `openUndoWindow` refuses a count of zero.
   */
  async arrangeBoard(scope: BoardScope): Promise<void> {
    const done = await this.board.arrange(scope);
    if (!done) return;

    this.openUndoWindow({ kind: 'arrange', layout: done.previous, count: done.moved });
  }

  /** Outside `edit()`: filling a field is not editing the note, so `updatedAt` stays put. */
  async setPlaceholderValues(id: string, values: Record<string, string>): Promise<void> {
    const resolved = await this.resolve(id);
    const target = resolved === DRAFT_ID ? await this.materialiseDraft() : resolved;
    if (!target) return;

    const saved = await this.notifier.attempt('errors.noteSaveFailed', () =>
      this.repository.setPlaceholderValues(target, values),
    );
    if (!saved) return;

    if (this.persistedNoteId() === target) {
      this._selectedNote.set(saved);
    }
    this.revision.bump();
  }

  fillPlaceholders(content: string, values: Record<string, string>): Promise<string> {
    return this.repository.fillPlaceholders(content, values);
  }

  private spaceForNewNote(): string | null {
    const spaceId = this.spaces.activeSpaceId() ?? this.spaces.spaces()[0]?.id;
    if (!spaceId) {
      this.notifier.notify({ ref: { key: 'errors.spaceRequired' } });
      return null;
    }

    return spaceId;
  }

  /** ⚠️ Synchronous down to the assignment, so a second caller joins the write in flight. */
  private saveDraft(draft: Note): Promise<string | null> {
    this.draftMaterialisation ??= this.writeDraft(draft);

    return this.draftMaterialisation;
  }

  /** A refused write releases the gate: the next commit may still be worth keeping. */
  private async writeDraft(draft: Note): Promise<string | null> {
    const created = await this.persistNew(toDraftPayload(draft));
    if (!created) {
      this.draftMaterialisation = null;
      return null;
    }

    // Before the draft is dropped: between the two, `find()` would know the note by
    // neither name.
    this.materialisedNote = created;
    this._draftNote.set(null);

    return created.id;
  }

  private async persistNew(payload: NoteDraft): Promise<Note | null> {
    // ⚠️ Read before the write leaves: the editor can be closed inside the round trip,
    // and adopting the created note would then put the overlay back on screen.
    const session = this._editorSession();

    const created = await this.notifier.attempt('errors.noteCreateFailed', () =>
      this.repository.create(payload),
    );
    if (!created) return null;

    // Only the adoption is conditional: the canvas has to learn about the note either way.
    if (this._editorSession() === session) {
      this._selectedNote.set(created);
      this.selection.focusNote(created.id);
    }

    this.revision.bump();
    return created;
  }

  private discardDraft(): void {
    this._draftNote.set(null);
    this.draftMaterialisation = null;
  }

  /** `null` when there was nothing to act on, or when the batch failed. */
  private async runOnSelection<T>(action: (ids: readonly string[]) => Promise<T>): Promise<T | null> {
    const ids = this.selection.checkedNoteIds();
    if (ids.length === 0) return null;

    const done = await this.notifier.attempt('errors.bulkActionFailed', () => action(ids));
    if (done !== null) this.revision.bump();

    return done;
  }

  /** ⚠️ The banner fades, the action stays undoable: `Ctrl+Z` still works once it is gone. */
  private openUndoWindow(action: Reversible): void {
    // Nothing moved is nothing to offer: a bar saying "0 notes" is noise, not an undo.
    if (action.count === 0) return;

    this._lastAction.set(action);
    this._undoVisible.set(true);
    this.hideUndoBanner(undefined);
  }

  /** `changes` answers `null` when nothing moved: a no-op edit makes no round trip. */
  private async edit(id: string, changes: (note: Note) => NotePatch | null): Promise<void> {
    const resolved = await this.resolve(id);
    const target = this.find(resolved);
    const patch = target && changes(target);
    if (!patch) return;

    if (resolved === DRAFT_ID) {
      await this.editDraft(target, patch);
      return;
    }

    await this.persist(resolved, patch);
  }

  /** A write on a draft stays local while the note is not worth keeping. */
  private async editDraft(draft: Note, patch: NotePatch): Promise<void> {
    const updated: Note = { ...draft, ...patch };

    if (!isWorthSaving(updated)) {
      this._draftNote.set(updated);
      return;
    }

    // ⚠️ A write in flight owns the row, so this patch updates it rather than creating a
    // second one. Joining the write would drop it: that carries the first call's argument.
    if (this.draftMaterialisation) {
      const id = await this.draftMaterialisation;

      return id ? this.persist(id, patch) : undefined;
    }

    await this.saveDraft(updated);
  }

  /**
   * `DRAFT_ID` names the draft **or** the note it became: the editor chains commits
   * with no change detection between them and keeps sending the old id. ⚠️ Async on
   * purpose — awaiting the write in flight is what makes the next commit an update.
   */
  private async resolve(id: string): Promise<string> {
    if (id !== DRAFT_ID || !this.draftMaterialisation) return id;

    return (await this.draftMaterialisation) ?? DRAFT_ID;
  }

  private async persist(id: string, patch: NotePatch): Promise<void> {
    const saved = await this.notifier.attempt('errors.noteSaveFailed', () =>
      this.repository.update(id, patch),
    );
    if (!saved) return;

    if (this.persistedNoteId() === id) {
      this._selectedNote.set(saved);
    }
    // Kept current, so later commits compare against what was written rather than
    // against the payload the row was created with.
    if (this.materialisedNote?.id === id) {
      this.materialisedNote = saved;
    }
    this.revision.bump();
  }

  /** The open note first: it may have left the filtered view without ceasing to be editable. */
  private find(id: string): Note | null {
    const draft = this._draftNote();
    if (draft?.id === id) return draft;

    const selected = this._selectedNote();
    if (selected?.id === id) return selected;

    const materialised = this.materialisedNote;
    if (materialised?.id === id) return materialised;

    // ⚠️ The board too, and not the canvas alone: it **dims** where the canvas **narrows**,
    // so a card there can be ticked, moved or deleted while its note is nowhere in the
    // canvas view — and an unresolved note is a gesture that writes nothing, silently.
    return this.notes.findVisible(id) ?? this.board.findVisible(id);
  }
}
