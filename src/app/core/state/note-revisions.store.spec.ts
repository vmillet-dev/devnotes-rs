import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createNote } from '@testing/note.fixture';
import { FakeNotesRepository } from '@testing/fake-notes-repository';
import { provideAppTesting } from '@testing/testing.providers';
import { NoteRevisionsStore } from './note-revisions.store';
import { NotesRevision } from './notes-revision';
import { NotesStore } from './notes.store';

describe('NoteRevisionsStore', () => {
  let repository: FakeNotesRepository;
  let store: NoteRevisionsStore;

  /** A note edited twice: `r-1` keeps "first", `r-2` keeps "second", "third" is current. */
  beforeEach(async () => {
    repository = new FakeNotesRepository([
      createNote({ id: 'n-1', content: 'first' }),
      createNote({ id: 'n-2', content: 'elsewhere' }),
    ]);
    await repository.update('n-1', { content: 'second' });
    await repository.update('n-1', { content: 'third' });

    TestBed.configureTestingModule({ providers: [provideAppTesting({ notesRepository: repository })] });
    store = TestBed.inject(NoteRevisionsStore);
  });

  it('lists the bodies kept beside the note it is opened for', async () => {
    await store.openFor('n-1');

    expect(store.revisions().map((revision) => revision.id)).toEqual(['r-2', 'r-1']);
    expect(store.hasHistory()).toBe(true);
  });

  it('starts again from nothing on another note, and on no note at all', async () => {
    await store.openFor('n-1');
    store.toggle();

    await store.openFor('n-2');
    expect(store.revisions()).toEqual([]);
    expect(store.isOpen()).toBe(false);

    await store.openFor('n-1');
    await store.openFor(null);
    expect(store.revisions()).toEqual([]);
  });

  it('previews a version against the current text, and counts the newer ones it would drop', async () => {
    await store.openFor('n-1');

    await store.openPreview('r-1');

    expect(store.preview()?.revision.id).toBe('r-1');
    expect(store.preview()?.newer).toBe(1);
    expect(store.preview()?.lines).toEqual([
      { kind: 'dropped', text: 'third' },
      { kind: 'restored', text: 'first' },
    ]);
  });

  /** ⚠️ A comparison that arrives after the editor moved on belongs to nobody. */
  it('drops a preview that arrives after the editor moved to another note', async () => {
    await store.openFor('n-1');

    const pending = store.openPreview('r-2');
    void store.openFor('n-2');
    await pending;

    expect(store.preview()).toBeNull();
  });

  it('restores nothing without a preview on screen', async () => {
    await store.openFor('n-1');
    const restore = vi.spyOn(repository, 'restoreRevision');

    await store.restore();

    expect(restore).not.toHaveBeenCalled();
  });

  /** ⚠️ The editor re-seeds its draft when the counter moves: the note must be there first. */
  it('adopts the restored note before it says a restore happened', async () => {
    await store.openFor('n-1');
    await store.openPreview('r-1');
    const seen: number[] = [];
    vi.spyOn(TestBed.inject(NotesStore), 'adoptRestored').mockImplementation(() => {
      seen.push(store.restored());
    });
    const bumps = vi.spyOn(TestBed.inject(NotesRevision), 'bump');

    await store.restore();

    expect(seen).toEqual([0]);
    expect(store.restored()).toBe(1);
    expect(bumps).toHaveBeenCalled();
    expect(store.preview()).toBeNull();
    expect(store.revisions()).toEqual([]);
  });

  it('closes a preview without restoring it', async () => {
    await store.openFor('n-1');
    await store.openPreview('r-2');

    store.closePreview();

    expect(store.preview()).toBeNull();
  });
});
