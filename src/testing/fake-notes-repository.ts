import { guard } from './fail-next';
import { NotesRepository } from '@core/data/notes.repository';
import { LanguageTag } from '@core/model/language.model';
import { DiffLine, Revision } from '@core/model/revision.model';
import {
  Note,
  NoteDraft,
  NotePatch,
  NotePlacement,
  NotesQuery,
  NotesView,
  NoteTag,
  SampleNote,
  TagUsage,
  TrashedNote,
} from '@core/model/note.model';
import { Space } from '@core/model/space.model';
import { byCodeUnit } from '@core/utils/order.util';
import { checklistMarkdown } from './note.fixture';

/** Mirrors `notes::trash::RETENTION`, so the double's `purgeAt` is plausible. */
const RETENTION_MS = 30 * 24 * 60 * 60 * 1000;

/**
 * ⚠️ Mirrors `note_tags.tag COLLATE NOCASE`, which SQLite folds over **ASCII only** —
 * `toLowerCase()` would fold more than the real column does, and the double would then
 * skip a tag the back end goes on to add.
 */
function sameTag(one: string, other: string): boolean {
  const fold = (value: string): string => value.replace(/[A-Z]/g, (letter) => letter.toLowerCase());

  return fold(one) === fold(other);
}

/**
 * ⚠️ It deliberately does not reimplement filtering, grouping, tag normalisation or
 * `{{field}}` parsing: those live in Rust, and a second copy would let a front-end spec
 * pass against rules the real back end does not apply. What it emulates is persistence,
 * plus a trivial single-section view a spec can replace with `setView`.
 *
 * `Pick<…, keyof …>` is the real class's public surface: a method renamed there fails
 * this file at compile time.
 */
export class FakeNotesRepository implements Pick<NotesRepository, keyof NotesRepository> {
  private notes: readonly Note[];
  private trashed: readonly TrashedNote[] = [];
  private forcedView: NotesView | null = null;
  private nextId = 0;

  /** When set, the next call to any method rejects with this error, then clears. */
  failNext: Error | null = null;

  /** What a spec sets: a paste reads as prose unless told otherwise. */
  detectedLanguage: LanguageTag = 'txt';

  /** Query the store sent last, for asserting how it assembles its parameters. */
  lastQuery: NotesQuery | null = null;
  queryCount = 0;

  movedTo: { ids: readonly string[]; spaceId: string } | null = null;
  taggedWith: { ids: readonly string[]; tags: readonly string[] } | null = null;
  retagged: { tags: readonly string[]; into: string } | null = null;
  deletedTags: string[] = [];

  private variables: Record<string, string> = {};

  private gate: Promise<void> | null = null;
  private openGate: (() => void) | null = null;

  /** Records what the first launch asked for, and files the notes into the space it names. */
  seededSamples: {
    spaceName: string;
    folders: readonly string[];
    notes: readonly SampleNote[];
  } | null = null;

  private readonly history = new Map<string, readonly Revision[]>();
  private readonly bodies = new Map<string, string>();
  private nextRevision = 0;

  constructor(notes: readonly Note[] = []) {
    this.notes = notes;
  }

  /** Pins the view the backend is pretending to return, ignoring the stored notes. */
  setView(view: Partial<NotesView>): void {
    this.forcedView = { ...this.trivialView(), ...view };
  }

  /** Suspends every query until `release()`, so a spec can observe the in-flight state. */
  hold(): void {
    this.gate = new Promise<void>((resolve) => (this.openGate = resolve));
  }

  release(): void {
    this.openGate?.();
    this.gate = null;
    this.openGate = null;
  }

  async query(query: NotesQuery): Promise<NotesView> {
    await this.gate;
    return guard(this, () => {
      this.lastQuery = query;
      this.queryCount += 1;
      return this.forcedView ?? this.trivialView(query);
    });
  }

  get(id: string): Promise<Note> {
    return guard(this, () => {
      const note = this.notes.find((each) => each.id === id);
      if (!note) {
        throw new Error(`Unknown note: ${id}`);
      }
      return note;
    });
  }

  /** The stored notes are whole; a spec hands a truncated one over through `setView`. */
  async whole(note: Note): Promise<Note> {
    return note.truncated ? this.get(note.id) : note;
  }

  create(draft: NoteDraft): Promise<Note> {
    return guard(this, () => {
      const now = new Date();
      const note: Note = {
        ...draft,
        id: `fake-${++this.nextId}`,
        createdAt: now,
        updatedAt: now,
        footer: { kind: 'age', at: now },
        expiringSoon: false,
        folder: null,
        placeholders: [],
        attachmentCount: 0,
        copyText: draft.kind === 'checklist' ? checklistMarkdown(draft.items) : null,
        searchHit: null,
        truncated: false,
      };
      this.notes = [note, ...this.notes];
      return note;
    });
  }

  detectLanguage(): Promise<LanguageTag> {
    return guard(this, () => this.detectedLanguage);
  }

  duplicate(id: string, title: string): Promise<Note> {
    return guard(this, () => {
      const original = this.notes.find((each) => each.id === id);
      if (!original) {
        throw new Error(`Unknown note: ${id}`);
      }
      const now = new Date();
      const copy: Note = {
        ...original,
        id: `fake-${++this.nextId}`,
        title,
        pinned: false,
        createdAt: now,
        updatedAt: now,
      };
      this.notes = [copy, ...this.notes];
      return copy;
    });
  }

  seedSamples(spaceName: string, folders: readonly string[], notes: readonly SampleNote[]): Promise<Space> {
    return guard(this, () => {
      const spaceId = `fake-space-${++this.nextId}`;
      this.seededSamples = { spaceName, folders, notes };
      for (const note of notes) {
        // The index is resolved by the engine; the double only has to keep it legible.
        const folderId = note.folder === undefined ? null : `fake-folder-${note.folder}`;
        void this.create({ ...note.draft, spaceId, folderId });
      }
      return { id: spaceId, name: spaceName, pinned: false };
    });
  }

  update(id: string, patch: NotePatch): Promise<Note> {
    return guard(this, () => {
      const existing = this.notes.find((note) => note.id === id);
      if (!existing) {
        throw new Error(`Unknown note: ${id}`);
      }
      // ⚠️ The body **before** the edit, like the real one: the version that worked —
      // and never an empty one, which is what a note is born with.
      if (patch.content !== undefined && patch.content !== existing.content && existing.content !== '') {
        this.history.set(id, [
          { id: `r-${++this.nextRevision}`, takenAt: new Date(), characters: existing.content.length },
          ...(this.history.get(id) ?? []),
        ]);
        this.bodies.set(`r-${this.nextRevision}`, existing.content);
      }
      const updated: Note = { ...existing, ...patch, updatedAt: new Date() };
      this.notes = this.notes.map((note) => (note.id === id ? updated : note));
      return updated;
    });
  }

  listRevisions(id: string): Promise<readonly Revision[]> {
    return guard(this, () => this.history.get(id) ?? []);
  }

  /** Not a real diff: the whole current text goes, the whole version comes back. */
  revisionDiff(id: string, revisionId: string): Promise<readonly DiffLine[]> {
    return guard(this, () => {
      const current = this.notes.find((note) => note.id === id)?.content;
      const version = this.bodies.get(revisionId);
      if (current === undefined || version === undefined) {
        throw new Error(`Unknown revision: ${revisionId}`);
      }

      return current === version
        ? []
        : [
            { kind: 'dropped', text: current },
            { kind: 'restored', text: version },
          ];
    });
  }

  /**
   * Going back, like the real one: the version and every newer one leave the history, and
   * nothing is kept of the text replaced. ⚠️ `updatedAt` is left alone.
   */
  restoreRevision(id: string, revisionId: string): Promise<Note> {
    return guard(this, () => {
      const existing = this.notes.find((note) => note.id === id);
      if (!existing) {
        throw new Error(`Unknown note: ${id}`);
      }
      const content = this.bodies.get(revisionId);
      if (content === undefined) {
        throw new Error(`Unknown revision: ${revisionId}`);
      }

      const kept = this.history.get(id) ?? [];
      const at = kept.findIndex((revision) => revision.id === revisionId);
      this.history.set(id, kept.slice(at + 1));

      const restored: Note = { ...existing, content };
      this.notes = this.notes.map((note) => (note.id === id ? restored : note));
      return restored;
    });
  }

  delete(id: string): Promise<void> {
    return guard(this, () => {
      this.trash([id]);
    });
  }

  deleteMany(ids: readonly string[]): Promise<number> {
    return guard(this, () => this.trash(ids));
  }

  restore(ids: readonly string[]): Promise<number> {
    return guard(this, () => {
      const restored = this.trashed.filter((note) => ids.includes(note.id));
      this.trashed = this.trashed.filter((note) => !ids.includes(note.id));
      this.notes = [
        ...restored.map((note) => ({
          ...note,
          updatedAt: note.deletedAt,
          createdAt: note.deletedAt,
          pinned: false,
          source: '',
          lifecycle: { kind: 'permanent' } as const,
          footer: { kind: 'age', at: note.deletedAt } as const,
          expiringSoon: false,
          placeholders: [],
          attachmentCount: 0,
          // The trash shape carries no folder: a restored note comes back loose.
          folderId: null,
          folder: null,
          copyText: null,
          searchHit: null,
          truncated: false,
          // The trash shape drops the items; a spec needing them restored uses `setView`.
          items: [],
        })),
        ...this.notes,
      ];
      return restored.length;
    });
  }

  loadTrash(): Promise<readonly TrashedNote[]> {
    return guard(this, () => this.trashed);
  }

  purge(ids: readonly string[]): Promise<number> {
    return guard(this, () => {
      const before = this.trashed.length;
      this.trashed = this.trashed.filter((note) => !ids.includes(note.id));
      return before - this.trashed.length;
    });
  }

  emptyTrash(): Promise<number> {
    return guard(this, () => {
      const count = this.trashed.length;
      this.trashed = [];
      return count;
    });
  }

  /** What the double actually holds, so a spec can assert an undo really put it back. */
  spaceOf(id: string): string | undefined {
    return this.notes.find((note) => note.id === id)?.spaceId;
  }

  contentOf(id: string): string | undefined {
    return this.notes.find((note) => note.id === id)?.content;
  }

  tagsOf(id: string): readonly string[] | undefined {
    return this.notes.find((note) => note.id === id)?.tags;
  }

  moveMany(ids: readonly string[], spaceId: string): Promise<readonly NotePlacement[]> {
    return guard(this, () => {
      this.movedTo = { ids, spaceId };
      // Only the ones that actually change space, like `notes::store::move_many`.
      const previous = this.notes
        .filter((note) => ids.includes(note.id) && note.spaceId !== spaceId)
        .map((note) => ({ noteId: note.id, spaceId: note.spaceId }));

      this.notes = this.notes.map((note) => (ids.includes(note.id) ? { ...note, spaceId } : note));

      return previous;
    });
  }

  moveBack(placements: readonly NotePlacement[]): Promise<number> {
    return guard(this, () => {
      const home = new Map(placements.map((placement) => [placement.noteId, placement.spaceId]));
      this.notes = this.notes.map((note) => {
        const spaceId = home.get(note.id);

        return spaceId === undefined ? note : { ...note, spaceId };
      });

      return placements.length;
    });
  }

  tagMany(ids: readonly string[], tags: readonly string[]): Promise<readonly NoteTag[]> {
    return guard(this, () => {
      this.taggedWith = { ids, tags };
      const added: NoteTag[] = [];

      this.notes = this.notes.map((note) => {
        if (!ids.includes(note.id)) return note;

        const missing = tags.filter((tag) => !note.tags.some((held) => sameTag(held, tag)));
        for (const tag of missing) {
          added.push({ noteId: note.id, tag });
        }

        return missing.length === 0 ? note : { ...note, tags: [...note.tags, ...missing] };
      });

      return added;
    });
  }

  untagMany(pairs: readonly NoteTag[]): Promise<number> {
    return guard(this, () => {
      let removed = 0;
      this.notes = this.notes.map((note) => {
        const strip = pairs.filter((pair) => pair.noteId === note.id);
        if (strip.length === 0) return note;

        const kept = note.tags.filter((tag) => !strip.some((pair) => sameTag(pair.tag, tag)));
        removed += note.tags.length - kept.length;

        return { ...note, tags: kept };
      });

      return removed;
    });
  }

  /** Trashed notes included, like the back end: a rename reaches them too. */
  countNotesTagged(tags: readonly string[]): Promise<number> {
    return guard(
      this,
      () =>
        [...this.notes, ...this.trashed].filter((note) =>
          note.tags.some((held) => tags.some((tag) => sameTag(held, tag))),
        ).length,
    );
  }

  loadTags(): Promise<readonly TagUsage[]> {
    return guard(this, () => {
      const counts = new Map<string, number>();
      for (const note of this.notes) {
        for (const tag of note.tags) {
          counts.set(tag, (counts.get(tag) ?? 0) + 1);
        }
      }
      return [...counts]
        .map(([tag, noteCount]) => ({ tag, noteCount }))
        .sort((a, b) => a.tag.localeCompare(b.tag));
    });
  }

  renameTags(tags: readonly string[], into: string): Promise<number> {
    return guard(this, () => {
      this.retagged = { tags, into };
      this.notes = this.notes.map((note) => ({
        ...note,
        tags: [...new Set(note.tags.map((tag) => (tags.includes(tag) ? into : tag)))],
      }));
      return tags.length;
    });
  }

  deleteTags(tags: readonly string[]): Promise<number> {
    return guard(this, () => {
      this.deletedTags.push(...tags);
      this.notes = this.notes.map((note) => ({
        ...note,
        tags: note.tags.filter((existing) => !tags.includes(existing)),
      }));
      return tags.length;
    });
  }

  /** ⚠️ `updatedAt` is left alone — that is the point of the command this stands for. */
  setPlaceholderValues(id: string, values: Record<string, string>): Promise<Note> {
    return guard(this, () => {
      const existing = this.notes.find((note) => note.id === id);
      if (!existing) {
        throw new Error(`Unknown note: ${id}`);
      }
      const updated: Note = {
        ...existing,
        // The fields come from the content, which only Rust parses.
        placeholders: existing.placeholders.map((placeholder) => ({
          ...placeholder,
          value: values[placeholder.name] ?? '',
        })),
      };
      this.notes = this.notes.map((note) => (note.id === id ? updated : note));
      return updated;
    });
  }

  /** The real one delegates to Rust; the double substitutes naively. */
  fillPlaceholders(content: string, values: Record<string, string>): Promise<string> {
    return guard(this, () =>
      Object.entries({ ...this.variables, ...values }).reduce(
        (filled, [name, value]) => filled.split(`{{${name}}}`).join(value),
        content,
      ),
    );
  }

  loadVariables(): Promise<Record<string, string>> {
    return guard(this, () => ({ ...this.variables }));
  }

  /** Empty values are dropped, exactly as `normalize_values` does in Rust. */
  saveVariables(values: Record<string, string>): Promise<Record<string, string>> {
    return guard(this, () => {
      this.variables = Object.fromEntries(
        Object.entries(values).filter(([name, value]) => name !== '' && value !== ''),
      );
      return { ...this.variables };
    });
  }

  private trash(ids: readonly string[]): number {
    const removed = this.notes.filter((note) => ids.includes(note.id));
    this.notes = this.notes.filter((note) => !ids.includes(note.id));

    const deletedAt = new Date();
    this.trashed = [
      ...removed.map((note) => ({
        id: note.id,
        spaceId: note.spaceId,
        title: note.title,
        language: note.language,
        content: note.content,
        tags: note.tags,
        kind: note.kind,
        deletedAt,
        purgeAt: new Date(deletedAt.getTime() + RETENTION_MS),
      })),
      ...this.trashed,
    ];

    return removed.length;
  }

  /**
   * Every note in one section — the shape, not the grouping rules. ⚠️ `isFiltering` is
   * answered the way the back end answers it, because the canvas reads it to decide
   * whether there is anything to clear.
   */
  private trivialView(query?: NotesQuery): NotesView {
    return {
      sections: [
        {
          key: 'week',
          notes: this.notes,
          hasExpiringNotes: this.notes.some((note) => note.lifecycle.kind === 'expires'),
          showCreateGhost: true,
        },
      ],
      availableTags: [...new Set(this.notes.flatMap((note) => note.tags))].sort(byCodeUnit),
      availableLanguages: [...new Set(this.notes.map((note) => note.language))].sort(byCodeUnit),
      isFiltering:
        (query?.search.trim().length ?? 0) > 0 ||
        (query?.tags.length ?? 0) > 0 ||
        (query?.languages.length ?? 0) > 0,
      matched: this.notes.length,
    };
  }
}
