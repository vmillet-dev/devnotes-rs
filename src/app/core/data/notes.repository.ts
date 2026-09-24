import { Injectable } from '@angular/core';
import { commands } from '@core/ipc/bindings';
import { unwrap } from '@core/ipc/ipc.error';
import { DiffLine, Revision } from '@core/model/revision.model';
import { Space } from '@core/model/space.model';
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
import {
  toNote,
  toWireNoteDraft,
  toWireNotePatch,
  toWireNotesQuery,
  toNotesView,
  toTrashedNote,
} from './note.mapper';

/**
 * ⚠️ `query` returns a view already filtered and grouped, and there is deliberately no
 * method handing back the raw list: one would invite re-filtering on the front.
 */
@Injectable({ providedIn: 'root' })
export class NotesRepository {
  async query(query: NotesQuery): Promise<NotesView> {
    return toNotesView(unwrap('query_notes', await commands.queryNotes(toWireNotesQuery(query))));
  }

  /** The whole note, by id: the editor's source, since a list sends previews. */
  async get(id: string): Promise<Note> {
    return toNote(unwrap('get_note', await commands.getNote(id)));
  }

  /** ⚠️ What a copy or a fill reads: a note taken from a list may carry its first lines only. */
  async whole(note: Note): Promise<Note> {
    return note.truncated ? this.get(note.id) : note;
  }

  async create(draft: NoteDraft): Promise<Note> {
    return toNote(unwrap('create_note', await commands.createNote(toWireNoteDraft(draft))));
  }

  /**
   * The first launch, as one write. Neither the space nor the folders exist yet, so a note
   * carries no space of its own and names its folder by **index** — the back end creates
   * all three in one transaction.
   */
  async seedSamples(
    spaceName: string,
    folders: readonly string[],
    notes: readonly SampleNote[],
  ): Promise<Space> {
    const wire = notes.map((note) => ({
      folder: note.folder ?? null,
      draft: toWireNoteDraft(note.draft),
    }));

    const space = unwrap('seed_samples', await commands.seedSamples(spaceName, [...folders], wire));
    return { ...space, pinned: space.pinned ?? false };
  }

  async update(id: string, patch: NotePatch): Promise<Note> {
    return toNote(unwrap('update_note', await commands.updateNote(id, toWireNotePatch(patch))));
  }

  /** The bodies kept beside a note, newest first. */
  async listRevisions(id: string): Promise<readonly Revision[]> {
    return unwrap('list_revisions', await commands.listRevisions(id)).map((wire) => ({
      ...wire,
      takenAt: new Date(wire.takenAt),
    }));
  }

  /** What going back to a kept body would change, against the current text. */
  async revisionDiff(id: string, revisionId: string): Promise<readonly DiffLine[]> {
    return unwrap('revision_diff', await commands.revisionDiff(id, revisionId));
  }

  /**
   * Goes back to a kept body; it and every body kept after it leave the history. ⚠️ Does
   * not refresh `updated_at` — putting something back is not editing it.
   */
  async restoreRevision(id: string, revisionId: string): Promise<Note> {
    return toNote(unwrap('restore_revision', await commands.restoreRevision(id, revisionId)));
  }

  /** Moves to the trash: the note is recoverable for 30 days. */
  async delete(id: string): Promise<void> {
    unwrap('delete_note', await commands.deleteNote(id));
  }

  async deleteMany(ids: readonly string[]): Promise<number> {
    return unwrap('delete_notes', await commands.deleteNotes([...ids]));
  }

  async restore(ids: readonly string[]): Promise<number> {
    return unwrap('restore_notes', await commands.restoreNotes([...ids]));
  }

  async loadTrash(): Promise<readonly TrashedNote[]> {
    return unwrap('list_trash', await commands.listTrash()).map(toTrashedNote);
  }

  async purge(ids: readonly string[]): Promise<number> {
    return unwrap('purge_notes', await commands.purgeNotes([...ids]));
  }

  async emptyTrash(): Promise<number> {
    return unwrap('empty_trash', await commands.emptyTrash());
  }

  /** Answers where each note came from, which is the only thing that can put it back. */
  async moveMany(ids: readonly string[], spaceId: string): Promise<readonly NotePlacement[]> {
    return unwrap('move_notes', await commands.moveNotes([...ids], spaceId));
  }

  async moveBack(placements: readonly NotePlacement[]): Promise<number> {
    return unwrap('move_notes_back', await commands.moveNotesBack([...placements]));
  }

  /** Answers the pairs it added, never the notes it merely looked at. */
  async tagMany(ids: readonly string[], tags: readonly string[]): Promise<readonly NoteTag[]> {
    return unwrap('tag_notes', await commands.tagNotes([...ids], [...tags]));
  }

  async untagMany(pairs: readonly NoteTag[]): Promise<number> {
    return unwrap('untag_notes', await commands.untagNotes([...pairs]));
  }

  /** What a corpus-wide tag action would touch, asked before it runs. */
  async countNotesTagged(tags: readonly string[]): Promise<number> {
    return unwrap('count_notes_tagged', await commands.countNotesTagged([...tags]));
  }

  async loadTags(): Promise<readonly TagUsage[]> {
    return unwrap('list_tags', await commands.listTags());
  }

  /** One tag or several: renaming onto an existing tag is a merge anyway. */
  async renameTags(tags: readonly string[], into: string): Promise<number> {
    return unwrap('rename_tags', await commands.renameTags([...tags], into));
  }

  async deleteTags(tags: readonly string[]): Promise<number> {
    return unwrap('delete_tags', await commands.deleteTags([...tags]));
  }

  /** `updatedAt` is unchanged: filling a field is not editing the note. */
  async setPlaceholderValues(id: string, values: Record<string, string>): Promise<Note> {
    return toNote(unwrap('set_placeholder_values', await commands.setPlaceholderValues(id, values)));
  }

  /** Global variables included, which is why this reads the database. */
  async fillPlaceholders(content: string, values: Record<string, string>): Promise<string> {
    return unwrap('fill_placeholders', await commands.fillPlaceholders(content, values));
  }

  async loadVariables(): Promise<Record<string, string>> {
    return unwrap('list_global_placeholders', await commands.listGlobalPlaceholders());
  }

  /** Stores the whole set. An empty value means "keep what the snippet offers". */
  async saveVariables(values: Record<string, string>): Promise<Record<string, string>> {
    return unwrap('set_global_placeholders', await commands.setGlobalPlaceholders(values));
  }
}
