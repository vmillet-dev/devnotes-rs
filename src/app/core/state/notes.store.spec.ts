import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { IpcError } from '@core/ipc/ipc.error';
import { FakeClipboard } from '@testing/fake-clipboard';
import { FakeNotesRepository } from '@testing/fake-notes-repository';
import { createNote } from '@testing/note.fixture';
import { HARNESS_SPACES, awaitQuery, createNotesHarness, visibleIds } from '@testing/notes-harness';
import { NotesRevision } from './notes-revision';
import { DRAFT_ID } from './notes.store';
import { UNDO_WINDOW_MS } from './undo.store';

describe('NotesStore', () => {
  beforeEach(() => {
    TestBed.resetTestingModule();
    vi.restoreAllMocks();
    // The stores report failures through console.error on purpose.
    vi.spyOn(console, 'error').mockImplementation(() => undefined);
  });

  describe('a draft closed while it was being written', () => {
    /** Holds `create` open so the editor can be closed with the write in flight. */
    function holdCreate(repository: FakeNotesRepository) {
      const write = repository.create.bind(repository);
      let open!: () => void;
      const held = new Promise<void>((resolve) => (open = resolve));

      const create = vi.spyOn(repository, 'create').mockImplementation(async (draft) => {
        await held;
        return write(draft);
      });

      return {
        inFlight: () => vi.waitFor(() => expect(create).toHaveBeenCalled()),
        land: () => open(),
      };
    }

    /**
     * ⚠️ The editor can be closed inside the round trip that materialises a draft, and
     * adopting the created note would then put the overlay back on screen.
     */
    it('does not put the editor back on screen', async () => {
      const { store, repository } = await createNotesHarness([]);
      const write = holdCreate(repository);

      store.createNote('snippet');
      const writing = store.applyPatch(DRAFT_ID, { title: 'Rotate the certificate' });
      await write.inFlight();

      store.closeOverlay();
      write.land();
      await writing;

      expect(store.selectedNote()).toBeNull();
    });

    /** ⚠️ The note is still written, and the canvas has to hear about it either way. */
    it('still puts the note on the canvas', async () => {
      const { store, canvas, repository } = await createNotesHarness([]);
      const write = holdCreate(repository);

      store.createNote('snippet');
      const writing = store.applyPatch(DRAFT_ID, { title: 'Rotate the certificate' });
      await write.inFlight();

      store.closeOverlay();
      write.land();
      await writing;

      await vi.waitFor(() => expect(visibleIds(canvas)).toHaveLength(1));
    });

    /**
     * ⚠️ The commits still in flight have to land on the row too. Closing fires title,
     * source and content back to back, the close lands between them, and a closed editor
     * adopts nothing — so without `materialisedNote` the later commits are dropped.
     */
    it('lands the commits the close itself fired', async () => {
      const { store, repository } = await createNotesHarness([]);
      store.createNote('snippet');
      // The title is what materialises the row, and it is done being written.
      await store.applyPatch(DRAFT_ID, { title: 'Rotate the certificate' });

      const update = vi.spyOn(repository, 'update');

      // `requestClose()` fires the commits and closes in the same turn, so the close
      // lands while they are still suspended on the draft's resolution.
      const writing = store.applyPatch(DRAFT_ID, { content: 'openssl req -new' });
      store.closeOverlay();
      await writing;

      expect(update).toHaveBeenCalledWith(expect.any(String), { content: 'openssl req -new' });
    });
  });

  describe('selection', () => {
    it('exposes the selected note and clears it on close', async () => {
      const note = createNote({ id: 'selected' });
      const { store } = await createNotesHarness([note]);

      store.openNote('selected');
      expect(store.selectedNote()).toEqual(note);

      store.closeOverlay();
      expect(store.selectedNote()).toBeNull();
    });

    it('returns null when the selected id does not match any note', async () => {
      const { store } = await createNotesHarness([createNote({ id: 'a' })]);

      store.openNote('does-not-exist');

      expect(store.selectedNote()).toBeNull();
    });

    it('keeps the open note editable after it drops out of the filtered view', async () => {
      const { store, canvas, repository } = await createNotesHarness([
        createNote({ id: 'a', tags: ['urgent'] }),
      ]);
      store.openNote('a');
      const before = repository.queryCount;

      repository.setView({ sections: [], matched: 0, isFiltering: true });
      canvas.toggleTag('other');
      await awaitQuery(repository, before);

      expect(visibleIds(canvas)).toEqual([]);
      expect(store.selectedNoteId()).toBe('a');
    });
  });

  describe('mutations', () => {
    it('persists a pin toggle and adopts the stored note', async () => {
      const { store, repository } = await createNotesHarness([createNote({ id: 'a', pinned: false })]);
      const update = vi.spyOn(repository, 'update');
      store.openNote('a');

      await store.togglePinned('a');

      expect(update).toHaveBeenCalledWith('a', { pinned: true });
      expect(store.selectedNote()?.pinned).toBe(true);
    });

    it('re-queries the backend after a successful write', async () => {
      const { store, repository } = await createNotesHarness([createNote({ id: 'a', title: 'Old' })]);
      const before = repository.queryCount;

      await store.applyPatch('a', { title: 'New' });
      await awaitQuery(repository, before);

      expect(repository.queryCount).toBeGreaterThan(before);
    });

    /**
     * ⚠️ Through `NotesRevision` and not by reloading the canvas by hand: the board is a
     * second view of the same notes, and a write it never hears about leaves a ticked
     * item looking unticked until the view is switched.
     */
    it('bumps the revision, so the second view re-reads too', async () => {
      const { store } = await createNotesHarness([createNote({ id: 'a', title: 'Old' })]);
      const revision = TestBed.inject(NotesRevision);
      const before = revision.current();

      await store.applyPatch('a', { title: 'New' });

      expect(revision.current()).toBeGreaterThan(before);
    });

    it('skips persistence when the value has not changed', async () => {
      const { store, repository } = await createNotesHarness([createNote({ id: 'a', title: 'Same' })]);
      const update = vi.spyOn(repository, 'update');

      await store.applyPatch('a', { title: 'Same' });
      await store.applyPatch('a', { content: 'line one\nline two' });
      await store.applyPatch('a', { language: 'txt' });

      expect(update).not.toHaveBeenCalled();
    });

    it('notifies and leaves the view alone when a write fails', async () => {
      const { store, canvas, repository } = await createNotesHarness([
        createNote({ id: 'a', title: 'Original' }),
      ]);
      const notifier = TestBed.inject(ErrorNotifier);
      repository.failNext = new Error('disk full');

      await store.applyPatch('a', { title: 'Attempted' });

      expect(visibleIds(canvas)).toEqual(['a']);
      expect(canvas.sections()[0].notes[0].title).toBe('Original');
      expect(notifier.notice()?.ref.key).toBe('errors.noteSaveFailed');
    });

    it('does nothing for an unknown id', async () => {
      const { store, repository } = await createNotesHarness([createNote({ id: 'a' })]);
      const update = vi.spyOn(repository, 'update');
      const remove = vi.spyOn(repository, 'delete');

      await store.togglePinned('missing');
      await store.deleteNote('missing');

      expect(update).not.toHaveBeenCalled();
      expect(remove).not.toHaveBeenCalled();
    });

    it('writes a tag list that differs from the stored one', async () => {
      const { store, repository } = await createNotesHarness([
        createNote({ id: 'a', tags: ['keep', 'drop'] }),
      ]);
      const update = vi.spyOn(repository, 'update');

      await store.applyPatch('a', { tags: ['keep'] });

      expect(update).toHaveBeenCalledWith('a', { tags: ['keep'] });
    });

    it('does nothing when the tag list comes back the same', async () => {
      const { store, repository } = await createNotesHarness([createNote({ id: 'a', tags: ['keep'] })]);
      const update = vi.spyOn(repository, 'update');

      await store.applyPatch('a', { tags: ['keep'] });

      expect(update).not.toHaveBeenCalled();
    });

    it('drops the fields that have not moved and keeps the rest', async () => {
      const { store, repository } = await createNotesHarness([
        createNote({ id: 'a', title: 'Same', content: 'old' }),
      ]);
      const update = vi.spyOn(repository, 'update');

      await store.applyPatch('a', { title: 'Same', content: 'new' });

      expect(update).toHaveBeenCalledWith('a', { content: 'new' });
    });
  });

  describe('moveNote', () => {
    it('files the note in another space', async () => {
      const { store, repository } = await createNotesHarness([createNote({ id: 'a', spaceId: 'space-1' })]);
      const update = vi.spyOn(repository, 'update');

      await store.moveNote('a', 'space-2');

      expect(update).toHaveBeenCalledWith('a', { spaceId: 'space-2' });
    });

    it('does not write when the note is already in that space', async () => {
      const { store, repository } = await createNotesHarness([createNote({ id: 'a', spaceId: 'space-1' })]);
      const update = vi.spyOn(repository, 'update');

      await store.moveNote('a', 'space-1');

      expect(update).not.toHaveBeenCalled();
    });

    it('reports a space that no longer exists', async () => {
      const { store, repository } = await createNotesHarness([createNote({ id: 'a', spaceId: 'space-1' })]);
      const notifier = TestBed.inject(ErrorNotifier);
      repository.failNext = new IpcError('update_note', {
        code: 'spaceNotFound',
        params: { id: 'space-2' },
        detail: 'Espace introuvable : space-2',
      });

      await store.moveNote('a', 'space-2');

      expect(notifier.notice()?.ref.key).toBe('errors.spaceGone');
    });
  });

  describe('setChecklist', () => {
    const items = [
      { text: 'Relire', done: false },
      { text: 'Déployer', done: false },
    ];

    it('replaces the whole list, no item having an identity of its own', async () => {
      const { store, repository } = await createNotesHarness([
        createNote({ id: 'a', kind: 'checklist', items }),
      ]);
      const update = vi.spyOn(repository, 'update');

      await store.setChecklist('a', [{ text: 'Relire', done: true }]);

      expect(update).toHaveBeenCalledWith('a', { items: [{ text: 'Relire', done: true }] });
    });

    it('does not write when nothing changed', async () => {
      const { store, repository } = await createNotesHarness([
        createNote({ id: 'a', kind: 'checklist', items }),
      ]);
      const update = vi.spyOn(repository, 'update');

      await store.setChecklist(
        'a',
        items.map((item) => ({ ...item })),
      );

      expect(update).not.toHaveBeenCalled();
    });

    /**
     * ⚠️ The board dims where the canvas narrows: a card there can be ticked while its
     * note is nowhere in the canvas view, and resolving it against that view alone made
     * the click write nothing at all.
     */
    it('ticks a card the canvas has filtered out but the board is showing', async () => {
      const note = createNote({ id: 'a', kind: 'checklist', items });
      const { store, selection, repository } = await createNotesHarness([]);
      const update = vi.spyOn(repository, 'update');
      vi.spyOn(selection, 'noteOnScreen').mockReturnValue(note);

      await store.setChecklist('a', [{ text: 'Relire', done: true }]);

      expect(update).toHaveBeenCalledWith('a', { items: [{ text: 'Relire', done: true }] });
    });

    it('writes when only the order changed', async () => {
      const { store, repository } = await createNotesHarness([
        createNote({ id: 'a', kind: 'checklist', items }),
      ]);
      const update = vi.spyOn(repository, 'update');

      await store.setChecklist('a', [items[1], items[0]]);

      expect(update).toHaveBeenCalledTimes(1);
    });
  });

  describe('deadline', () => {
    it('turns a permanent note into an expiring one', async () => {
      const { store, repository } = await createNotesHarness([createNote({ id: 'a' })]);
      const update = vi.spyOn(repository, 'update');
      const at = new Date('2026-08-01T23:59:59.999Z');

      await store.applyPatch('a', { lifecycle: { kind: 'expires', at } });

      expect(update).toHaveBeenCalledWith('a', { lifecycle: { kind: 'expires', at } });
    });

    it('clears an expiry back to permanent', async () => {
      const { store, repository } = await createNotesHarness([
        createNote({ id: 'a', lifecycle: { kind: 'expires', at: new Date('2026-08-01T00:00:00Z') } }),
      ]);
      const update = vi.spyOn(repository, 'update');

      await store.applyPatch('a', { lifecycle: { kind: 'permanent' } });

      expect(update).toHaveBeenCalledWith('a', { lifecycle: { kind: 'permanent' } });
    });

    it('does not write when the deadline is unchanged', async () => {
      const at = new Date('2026-08-01T00:00:00Z');
      const { store, repository } = await createNotesHarness([
        createNote({ id: 'a', lifecycle: { kind: 'expires', at } }),
      ]);
      const update = vi.spyOn(repository, 'update');

      await store.applyPatch('a', { lifecycle: { kind: 'expires', at: new Date(at.getTime()) } });

      expect(update).not.toHaveBeenCalled();
    });

    it('writes when only the deadline moves', async () => {
      const { store, repository } = await createNotesHarness([
        createNote({ id: 'a', lifecycle: { kind: 'expires', at: new Date('2026-08-01T00:00:00Z') } }),
      ]);
      const update = vi.spyOn(repository, 'update');

      await store.applyPatch('a', { lifecycle: { kind: 'expires', at: new Date('2026-09-01T00:00:00Z') } });

      expect(update).toHaveBeenCalled();
    });

    it('stores the context a note is given', async () => {
      const { store, repository } = await createNotesHarness([createNote({ id: 'a', source: '' })]);
      const update = vi.spyOn(repository, 'update');

      await store.applyPatch('a', { source: 'API Gateway / Auth' });

      expect(update).toHaveBeenCalledWith('a', { source: 'API Gateway / Auth' });
    });

    it('does not write an unchanged context', async () => {
      const { store, repository } = await createNotesHarness([createNote({ id: 'a', source: 'API' })]);
      const update = vi.spyOn(repository, 'update');

      await store.applyPatch('a', { source: 'API' });

      expect(update).not.toHaveBeenCalled();
    });
  });

  describe('captureFromClipboard', () => {
    it('creates a note carrying what the clipboard held', async () => {
      const { store } = await createNotesHarness([], HARNESS_SPACES, new FakeClipboard('SELECT 1'));

      await store.captureFromClipboard();

      expect(store.selectedNote()).toMatchObject({ id: 'fake-1', content: 'SELECT 1' });
    });

    it('leaves the language for the backend to detect', async () => {
      const { store, repository } = await createNotesHarness(
        [],
        HARNESS_SPACES,
        new FakeClipboard('{"a": 1}'),
      );
      const create = vi.spyOn(repository, 'create');

      await store.captureFromClipboard();

      expect(create).toHaveBeenCalledWith(expect.objectContaining({ language: 'txt' }));
    });

    it('creates nothing from an empty or blank clipboard', async () => {
      const { store, repository } = await createNotesHarness([], HARNESS_SPACES, new FakeClipboard('  \n '));
      const create = vi.spyOn(repository, 'create');

      await store.captureFromClipboard();

      expect(create).not.toHaveBeenCalled();
      expect(store.selectedNote()).toBeNull();
    });

    it('creates nothing when the clipboard cannot be read', async () => {
      const clipboard = new FakeClipboard('SELECT 1');
      clipboard.failNext = new Error('no plugin');
      const { store, repository } = await createNotesHarness([], HARNESS_SPACES, clipboard);
      const create = vi.spyOn(repository, 'create');

      await store.captureFromClipboard();

      expect(create).not.toHaveBeenCalled();
    });

    it('opens the captured note so it can be titled straight away', async () => {
      const { store } = await createNotesHarness([], HARNESS_SPACES, new FakeClipboard('kubectl rollout'));

      await store.captureFromClipboard();

      expect(store.selectedNoteId()).toBe('fake-1');
    });
  });

  describe('createNote', () => {
    it('writes nothing until the note is worth keeping', async () => {
      const { store, repository } = await createNotesHarness([]);
      const create = vi.spyOn(repository, 'create');

      store.createNote();

      expect(create).not.toHaveBeenCalled();
      expect(store.selectedNoteId()).toBe(DRAFT_ID);
      expect(store.persistedNoteId()).toBeNull();
    });

    it('opens a blank draft', async () => {
      const { store } = await createNotesHarness([]);

      store.createNote();

      expect(store.selectedNote()).toMatchObject({
        id: DRAFT_ID,
        title: '',
        source: '',
        content: '',
        pinned: false,
        tags: [],
      });
    });

    it('opens a checklist draft when the menu asks for one', async () => {
      const { store } = await createNotesHarness([]);

      store.createNote('checklist');

      expect(store.selectedNote()).toMatchObject({ id: DRAFT_ID, kind: 'checklist', items: [] });
    });

    it('creates an ordinary note when nothing says otherwise', async () => {
      const { store } = await createNotesHarness([]);

      store.createNote();

      expect(store.selectedNote()?.kind).toBe('snippet');
    });

    it('keeps an empty checklist draft local', async () => {
      const { store, repository } = await createNotesHarness([]);
      const create = vi.spyOn(repository, 'create');

      store.createNote('checklist');

      expect(create).not.toHaveBeenCalled();
    });

    /**
     * ⚠️ `requestClose()` fires three commits back to back with no `await` between them,
     * so a draft that still resolves to `DRAFT_ID` in that window is created twice.
     */
    it('creates one note when the closing commits are chained without awaiting', async () => {
      const { store, repository } = await createNotesHarness([]);
      const written = repository.create.bind(repository);
      let release!: () => void;
      const held = new Promise<void>((resolve) => (release = resolve));
      const create = vi.spyOn(repository, 'create').mockImplementation(async (draft) => {
        await held;
        return written(draft);
      });
      const update = vi.spyOn(repository, 'update');

      store.createNote();
      const commits = [
        store.applyPatch(DRAFT_ID, { title: 'Staging CSR' }),
        store.applyPatch(DRAFT_ID, { content: 'openssl req -new -key staging.key' }),
      ];
      release();
      await Promise.all(commits);

      expect(create).toHaveBeenCalledTimes(1);
      // The second commit is not swallowed along the way: it lands as an update.
      expect(update).toHaveBeenCalledTimes(1);
      expect(store.persistedNoteId()).not.toBeNull();
    });

    it('keeps both fields when the closing commits are chained', async () => {
      const { store, repository } = await createNotesHarness([]);
      const written = repository.create.bind(repository);
      let release!: () => void;
      const held = new Promise<void>((resolve) => (release = resolve));
      vi.spyOn(repository, 'create').mockImplementation(async (draft) => {
        await held;
        return written(draft);
      });

      store.createNote();
      const commits = [
        store.applyPatch(DRAFT_ID, { title: 'Staging CSR' }),
        store.applyPatch(DRAFT_ID, { content: 'openssl req -new -key staging.key' }),
      ];
      release();
      await Promise.all(commits);

      // `persist` adopts what the back end returned, so the open note carries both.
      expect(store.selectedNote()).toMatchObject({
        title: 'Staging CSR',
        content: 'openssl req -new -key staging.key',
      });
    });

    it('persists a checklist as soon as it holds an item, having no body to fill', async () => {
      const { store, repository } = await createNotesHarness([]);
      const create = vi.spyOn(repository, 'create');
      store.createNote('checklist');

      await store.setChecklist(DRAFT_ID, [{ text: 'Relire', done: false }]);

      expect(create).toHaveBeenCalledTimes(1);
      expect(create.mock.calls[0][0]).toMatchObject({
        kind: 'checklist',
        items: [{ text: 'Relire', done: false }],
      });
      expect(store.persistedNoteId()).toBe('fake-1');
    });

    it('files the draft in the selected space', async () => {
      const { store, spaces } = await createNotesHarness([]);

      spaces.selectSpace('space-2');
      store.createNote();

      expect(store.selectedNote()?.spaceId).toBe('space-2');
    });

    it('files the draft in the first space while showing all spaces', async () => {
      const { store } = await createNotesHarness([]);

      store.createNote();

      expect(store.selectedNote()?.spaceId).toBe('space-1');
    });

    it('refuses to create a note when no space exists at all', async () => {
      const { store, repository } = await createNotesHarness([], []);
      const notifier = TestBed.inject(ErrorNotifier);
      const create = vi.spyOn(repository, 'create');

      store.createNote();

      expect(create).not.toHaveBeenCalled();
      expect(store.selectedNote()).toBeNull();
      expect(notifier.notice()?.ref.key).toBe('errors.spaceRequired');
    });

    it('persists on the first change worth keeping, and takes the real id', async () => {
      const { store } = await createNotesHarness([]);
      store.createNote();

      await store.applyPatch(DRAFT_ID, { title: 'Titre' });

      expect(store.persistedNoteId()).toBe('fake-1');
      expect(store.selectedNote()).toMatchObject({ id: 'fake-1', title: 'Titre' });
    });

    it('keeps a change that leaves the note empty local', async () => {
      const { store, repository } = await createNotesHarness([]);
      const create = vi.spyOn(repository, 'create');
      store.createNote();

      await store.applyPatch(DRAFT_ID, { language: 'json' });

      expect(create).not.toHaveBeenCalled();
      expect(store.selectedNote()).toMatchObject({ id: DRAFT_ID, language: 'json' });
    });

    it('saves a note that carries only a tag', async () => {
      const { store } = await createNotesHarness([]);
      store.createNote();

      await store.applyPatch(DRAFT_ID, { tags: ['urgent'] });

      expect(store.persistedNoteId()).toBe('fake-1');
    });

    it('routes a second commit still carrying the draft id to the real note', async () => {
      const { store, repository } = await createNotesHarness([]);
      const update = vi.spyOn(repository, 'update');
      store.createNote();

      await store.applyPatch(DRAFT_ID, { title: 'Titre' });
      await store.applyPatch(DRAFT_ID, { content: 'corps' });

      expect(update).toHaveBeenCalledWith('fake-1', { content: 'corps' });
      expect(store.selectedNote()?.content).toBe('corps');
    });

    it('discards an untouched draft on close', async () => {
      const { store, repository } = await createNotesHarness([]);
      const create = vi.spyOn(repository, 'create');
      store.createNote();

      store.closeOverlay();

      expect(create).not.toHaveBeenCalled();
      expect(store.selectedNote()).toBeNull();
    });

    it('discards the draft when another note is opened', async () => {
      const { store, repository } = await createNotesHarness([createNote({ id: 'a' })]);
      const create = vi.spyOn(repository, 'create');
      store.createNote();

      store.openNote('a');

      expect(create).not.toHaveBeenCalled();
      expect(store.selectedNoteId()).toBe('a');
    });

    it('throws nothing away and offers no undo when a draft is deleted', async () => {
      const { store, repository, undo } = await createNotesHarness([]);
      const remove = vi.spyOn(repository, 'delete');
      store.createNote();

      await store.deleteNote(DRAFT_ID);

      expect(remove).not.toHaveBeenCalled();
      expect(store.selectedNote()).toBeNull();
      expect(undo.last()).toBeNull();
    });

    it('materialises the draft on demand, for what needs a real note', async () => {
      const { store } = await createNotesHarness([]);
      store.createNote();

      expect(await store.materialiseDraft()).toBe('fake-1');
      expect(store.persistedNoteId()).toBe('fake-1');
    });

    it('has nothing to materialise without a draft', async () => {
      const { store } = await createNotesHarness([]);

      expect(await store.materialiseDraft()).toBeNull();
    });

    it('notifies and selects nothing when the write fails', async () => {
      const { store, repository } = await createNotesHarness([]);
      const notifier = TestBed.inject(ErrorNotifier);
      store.createNote();
      repository.failNext = new Error('read-only');

      await store.applyPatch(DRAFT_ID, { title: 'Titre' });

      expect(store.persistedNoteId()).toBeNull();
      expect(notifier.notice()?.ref.key).toBe('errors.noteCreateFailed');
    });
  });

  describe('deleteNote', () => {
    it('closes the overlay when the deleted note was the open one', async () => {
      const { store, canvas } = await createNotesHarness([createNote({ id: 'a' }), createNote({ id: 'b' })]);
      store.openNote('a');

      await store.deleteNote('a');

      expect(store.selectedNoteId()).toBeNull();
      await vi.waitFor(() => expect(visibleIds(canvas)).toEqual(['b']));
    });

    it('leaves the open note alone when another one is deleted', async () => {
      const { store } = await createNotesHarness([createNote({ id: 'a' }), createNote({ id: 'b' })]);
      store.openNote('a');

      await store.deleteNote('b');

      expect(store.selectedNoteId()).toBe('a');
    });

    it('keeps the note and notifies when deletion fails', async () => {
      const { store, canvas, repository } = await createNotesHarness([createNote({ id: 'a' })]);
      const notifier = TestBed.inject(ErrorNotifier);
      repository.failNext = new Error('locked');
      store.openNote('a');

      await store.deleteNote('a');

      expect(visibleIds(canvas)).toEqual(['a']);
      expect(store.selectedNoteId()).toBe('a');
      expect(notifier.notice()?.ref.key).toBe('errors.noteDeleteFailed');
    });
  });

  describe('trash and undo', () => {
    it('offers to undo what a deletion took away', async () => {
      const { store, undo } = await createNotesHarness([createNote({ id: 'a' })]);

      await store.deleteNote('a');

      expect(undo.last()).toEqual({ kind: 'deletion', ids: ['a'], count: 1 });
    });

    it('brings a deleted note back', async () => {
      const { store, canvas, undo } = await createNotesHarness([
        createNote({ id: 'a' }),
        createNote({ id: 'b' }),
      ]);
      await store.deleteNote('a');

      await undo.revert();
      await vi.waitFor(() => expect(visibleIds(canvas)).toContain('a'));

      expect(undo.last()).toBeNull();
    });

    it('clears the selection once it is in the trash', async () => {
      const { selection, batch, undo } = await createNotesHarness([
        createNote({ id: 'a' }),
        createNote({ id: 'b' }),
      ]);
      selection.toggleChecked('a');
      selection.toggleChecked('b');

      await batch.deleteSelection();

      expect(selection.hasSelection()).toBe(false);
      expect(undo.last()?.count).toBe(2);
    });

    it('hides the banner after its window but stays undoable', async () => {
      const { store, undo } = await createNotesHarness([createNote({ id: 'a' })]);
      // ⚠️ The timers are faked after the store is built: `waitFor` needs them to await
      // the first view.
      vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
      try {
        await store.deleteNote('a');
        expect(undo.banner()).not.toBeNull();

        await vi.advanceTimersByTimeAsync(UNDO_WINDOW_MS);

        expect(undo.banner()).toBeNull();
        expect(undo.last()).toEqual({ kind: 'deletion', ids: ['a'], count: 1 });
      } finally {
        vi.useRealTimers();
      }
    });

    it('drops the offer when dismissed', async () => {
      const { store, undo } = await createNotesHarness([createNote({ id: 'a' })]);
      await store.deleteNote('a');

      undo.dismiss();

      expect(undo.banner()).toBeNull();
      expect(undo.last()).toBeNull();
    });
  });

  describe('{{fields}}', () => {
    it('delegates the substitution to the backend', async () => {
      const { store } = await createNotesHarness();

      const filled = await store.fillPlaceholders('psql -h {{host}}', { host: 'db' });

      expect(filled).toBe('psql -h db');
    });

    it('saves the values and adopts the note that comes back', async () => {
      const snippet = createNote({
        id: 'snippet',
        content: 'psql -h {{host}}',
        placeholders: [{ name: 'host', defaultValue: '', value: '' }],
      });
      const { store } = await createNotesHarness([snippet]);
      store.openNote('snippet');

      await store.setPlaceholderValues('snippet', { host: 'db.internal' });

      expect(store.selectedNote()?.placeholders[0].value).toBe('db.internal');
    });

    it('gives the draft a real row before writing its values', async () => {
      const { store, repository } = await createNotesHarness();
      store.createNote();
      await store.applyPatch(DRAFT_ID, { content: 'psql -h {{host}}' });

      await store.setPlaceholderValues(DRAFT_ID, { host: 'db.internal' });

      expect(store.selectedNoteId()).not.toBe(DRAFT_ID);
      expect(repository.failNext).toBeNull();
    });

    it('reports a failed write rather than pretending it was kept', async () => {
      const snippet = createNote({
        id: 'snippet',
        placeholders: [{ name: 'host', defaultValue: '', value: '' }],
      });
      const { store, repository } = await createNotesHarness([snippet]);
      repository.failNext = new IpcError('set_placeholder_values', {
        code: 'noteNotFound',
        params: { id: 'snippet' },
        detail: 'Note introuvable : snippet',
      });

      await store.setPlaceholderValues('snippet', { host: 'db.internal' });

      expect(TestBed.inject(ErrorNotifier).notice()?.ref.key).toBe('errors.noteGone');
    });
  });
});

describe('NotesStore filing one note', () => {
  beforeEach(() => {
    TestBed.resetTestingModule();
    vi.restoreAllMocks();
    vi.spyOn(console, 'error').mockImplementation(() => undefined);
  });

  /** ⚠️ A batch of one, through the command the selection bar already takes. */
  it('files through the batch command and offers the filing back', async () => {
    const harness = await createNotesHarness(
      [createNote({ id: 'a', spaceId: 'space-1' })],
      undefined,
      undefined,
      [{ id: 'perf', spaceId: 'space-1', name: 'Perf', colour: 'amber', createdAt: new Date('2026-01-01') }],
    );

    await harness.store.fileNote('a', 'perf');

    expect(harness.folders.filings.get('a')).toBe('perf');
    expect(harness.undo.banner()).toMatchObject({ kind: 'file', count: 1 });
  });

  /**
   * ⚠️ `file_notes` answers the placements it changed, not the rows — so the open note
   * kept the folder it had, and the editor's own control went on naming it.
   */
  it('refreshes the open note, which the filing command does not answer with', async () => {
    const harness = await createNotesHarness(
      [createNote({ id: 'a', spaceId: 'space-1' })],
      undefined,
      undefined,
      [{ id: 'perf', spaceId: 'space-1', name: 'Perf', colour: 'amber', createdAt: new Date('2026-01-01') }],
    );
    harness.store.openNote('a');
    await vi.waitFor(() => expect(harness.store.selectedNote()?.id).toBe('a'));

    await harness.store.fileNote('a', 'perf');

    expect(harness.store.selectedNote()?.folderId).toBe('perf');
    expect(harness.store.selectedNote()?.folder?.name).toBe('Perf');
  });

  it('takes it back out, and forgets the folder with it', async () => {
    const harness = await createNotesHarness(
      [createNote({ id: 'a', spaceId: 'space-1', folderId: 'perf' })],
      undefined,
      undefined,
      [{ id: 'perf', spaceId: 'space-1', name: 'Perf', colour: 'amber', createdAt: new Date('2026-01-01') }],
    );
    harness.store.openNote('a');
    await vi.waitFor(() => expect(harness.store.selectedNote()?.id).toBe('a'));

    await harness.store.fileNote('a', null);

    expect(harness.store.selectedNote()?.folderId).toBeNull();
    expect(harness.store.selectedNote()?.folder).toBeNull();
  });
});
