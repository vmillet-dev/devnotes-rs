import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { SEARCH_DEBOUNCE_MS } from '@core/services/time/debounce';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { FakeNotesRepository } from '@testing/fake-notes-repository';
import { createNote } from '@testing/note.fixture';
import { provideAppTesting } from '@testing/testing.providers';
import { TrashStore } from './trash.store';

interface Harness {
  readonly store: TrashStore;
  readonly repository: FakeNotesRepository;
  readonly notifier: ErrorNotifier;
}

function createStore(): Harness {
  TestBed.resetTestingModule();
  const repository = new FakeNotesRepository([
    createNote({ id: 'note-1', title: 'First' }),
    createNote({ id: 'note-2', title: 'Second' }),
  ]);
  TestBed.configureTestingModule({ providers: [provideAppTesting({ notesRepository: repository })] });

  return {
    store: TestBed.inject(TrashStore),
    repository,
    notifier: TestBed.inject(ErrorNotifier),
  };
}

describe('TrashStore', () => {
  let harness: Harness;

  beforeEach(() => {
    harness = createStore();
  });

  it('loads nothing until the panel is opened', () => {
    // The trash is displayed nowhere else: a permanent resource would re-query on
    // every deletion.
    expect(harness.store.notes()).toEqual([]);
    expect(harness.store.isOpen()).toBe(false);
  });

  it('lists what has been deleted once opened', async () => {
    await harness.repository.delete('note-1');

    await harness.store.open();

    expect(harness.store.isOpen()).toBe(true);
    expect(harness.store.notes().map((note) => note.id)).toEqual(['note-1']);
  });

  it('asks Rust for what is typed, after the search pause, and opens on everything', async () => {
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
    try {
      await harness.repository.delete('note-1');
      await harness.repository.delete('note-2');
      await harness.store.open();

      harness.store.setSearch('sec');
      expect(harness.repository.lastTrashSearch).toBe('');
      await vi.advanceTimersByTimeAsync(SEARCH_DEBOUNCE_MS);

      expect(harness.repository.lastTrashSearch).toBe('sec');
      expect(harness.store.notes().map((note) => note.id)).toEqual(['note-2']);

      harness.store.close();
      await harness.store.open();
      expect(harness.store.search()).toBe('');
    } finally {
      vi.useRealTimers();
    }
  });

  it('restores everything at once, and says the canvas must reload', async () => {
    await harness.repository.delete('note-1');
    await harness.repository.delete('note-2');
    await harness.store.open();

    expect(await harness.store.restoreAll()).toBe(true);
    expect(harness.store.notes()).toEqual([]);
  });

  it('restores what the search found, and leaves the rest in the trash', async () => {
    await harness.repository.delete('note-1');
    await harness.repository.delete('note-2');
    await harness.store.open();
    harness.store.setSearch('sec');

    expect(await harness.store.restoreAll()).toBe(true);
    harness.store.setSearch('');
    await harness.store.open();

    expect(harness.store.notes().map((note) => note.id)).toEqual(['note-1']);
  });

  it('reports an empty trash distinctly from a loading one', async () => {
    await harness.store.open();

    expect(harness.store.isEmpty()).toBe(true);
  });

  it('restores a note and says the canvas must reload', async () => {
    await harness.repository.delete('note-1');
    await harness.store.open();

    expect(await harness.store.restore('note-1')).toBe(true);
    expect(harness.store.notes()).toEqual([]);
  });

  it('purges a single note without touching the others', async () => {
    await harness.repository.deleteMany(['note-1', 'note-2']);
    await harness.store.open();

    await harness.store.purge('note-1');

    expect(harness.store.notes().map((note) => note.id)).toEqual(['note-2']);
  });

  it('empties the trash in one go', async () => {
    await harness.repository.deleteMany(['note-1', 'note-2']);
    await harness.store.open();

    expect(await harness.store.emptyTrash()).toBe(true);
    expect(harness.store.notes()).toEqual([]);
  });

  it('surfaces a load failure instead of showing an empty trash', async () => {
    harness.repository.failNext = new Error('boom');

    await harness.store.open();

    expect(harness.notifier.notice()?.ref.key).toBe('errors.trashLoadFailed');
  });

  it('reports a failed action rather than claiming the canvas changed', async () => {
    await harness.repository.delete('note-1');
    await harness.store.open();
    harness.repository.failNext = new Error('boom');

    expect(await harness.store.restore('note-1')).toBe(false);
    expect(harness.notifier.notice()?.ref.key).toBe('errors.trashActionFailed');
  });
});
