import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { FakeNotesRepository } from '@testing/fake-notes-repository';
import { createNote } from '@testing/note.fixture';
import { provideAppTesting } from '@testing/testing.providers';
import { TagsStore } from './tags.store';

interface Harness {
  readonly store: TagsStore;
  readonly repository: FakeNotesRepository;
  readonly notifier: ErrorNotifier;
}

function createStore(): Harness {
  TestBed.resetTestingModule();
  const repository = new FakeNotesRepository([
    createNote({ id: 'note-1', tags: ['auth', 'api'] }),
    createNote({ id: 'note-2', tags: ['auth'] }),
  ]);
  TestBed.configureTestingModule({ providers: [provideAppTesting({ notesRepository: repository })] });

  return {
    store: TestBed.inject(TagsStore),
    repository,
    notifier: TestBed.inject(ErrorNotifier),
  };
}

describe('TagsStore', () => {
  let harness: Harness;

  beforeEach(() => {
    harness = createStore();
  });

  it('lists every tag of the corpus with its note count', async () => {
    await harness.store.open();

    expect(harness.store.tags()).toEqual([
      { tag: 'api', noteCount: 1 },
      { tag: 'auth', noteCount: 2 },
    ]);
  });

  it('starts each opening with an empty selection', async () => {
    await harness.store.open();
    harness.store.toggle('auth');
    harness.store.close();

    await harness.store.open();

    expect(harness.store.selectedCount()).toBe(0);
  });

  /**
   * Proposing must write nothing. This is the whole protection: the tag manager acts
   * on the corpus rather than on a selection, so a mis-click reaches it easily.
   */
  it('asks before it writes, and says how many notes it would touch', async () => {
    await harness.store.open();
    harness.store.toggle('auth');

    await harness.store.proposeRename('identity');

    expect(harness.store.pending()).toEqual({
      kind: 'rename',
      tags: ['auth'],
      into: 'identity',
      notes: 2,
    });
    expect(harness.repository.retagged).toBeNull();
  });

  it('renames a single selected tag once it is confirmed', async () => {
    await harness.store.open();
    harness.store.toggle('auth');
    await harness.store.proposeRename('identity');

    expect(await harness.store.confirm()).toBe(true);
    expect(harness.repository.retagged).toEqual({ tags: ['auth'], into: 'identity' });
    expect(harness.store.pending()).toBeNull();
  });

  it('merges when several tags are selected', async () => {
    // Renaming onto an existing tag *is* a merge in the database: the store only
    // picks the command whose name says so.
    await harness.store.open();
    harness.store.toggle('auth');
    harness.store.toggle('api');

    await harness.store.proposeRename('backend');

    expect(harness.store.pending()?.kind).toBe('merge');

    await harness.store.confirm();

    expect(harness.repository.retagged?.tags).toEqual(['auth', 'api']);
    expect(harness.repository.retagged?.into).toBe('backend');
  });

  /** A note carrying both tags is one note, not two. */
  it('counts the notes it would touch, rather than summing the per-tag counts', async () => {
    await harness.store.open();
    harness.store.toggle('auth');
    harness.store.toggle('api');

    await harness.store.proposeRename('backend');

    expect(harness.store.pending()?.notes).toBe(2);
  });

  it('withdraws the proposal when the selection changes under it', async () => {
    await harness.store.open();
    harness.store.toggle('auth');
    await harness.store.proposeRename('identity');

    harness.store.toggle('api');

    expect(harness.store.pending()).toBeNull();
  });

  it('cancelling changes nothing, which is the point of asking', async () => {
    await harness.store.open();
    harness.store.toggle('auth');
    await harness.store.proposeRename('identity');

    harness.store.cancel();

    expect(harness.store.pending()).toBeNull();
    expect(await harness.store.confirm()).toBe(false);
    expect(harness.repository.retagged).toBeNull();
  });

  it('refuses to rename towards nothing', async () => {
    await harness.store.open();
    harness.store.toggle('auth');

    await harness.store.proposeRename('   ');

    expect(harness.store.pending()).toBeNull();
  });

  it('does nothing without a selection', async () => {
    await harness.store.open();

    await harness.store.proposeRename('identity');
    await harness.store.proposeDelete();

    expect(harness.store.pending()).toBeNull();
  });

  it('drops every selected tag and clears the selection', async () => {
    await harness.store.open();
    harness.store.toggle('auth');
    harness.store.toggle('api');
    await harness.store.proposeDelete();

    expect(harness.store.pending()?.kind).toBe('delete');
    expect(await harness.store.confirm()).toBe(true);
    expect(harness.repository.deletedTags).toEqual(['auth', 'api']);
    expect(harness.store.selectedCount()).toBe(0);
    expect(harness.store.isEmpty()).toBe(true);
  });

  it('surfaces a load failure', async () => {
    harness.repository.failNext = new Error('boom');

    await harness.store.open();

    expect(harness.notifier.notice()?.ref.key).toBe('errors.tagsLoadFailed');
  });

  /** A blast radius that cannot be read is not a reason to go ahead blind. */
  it('proposes nothing when it cannot say what would be touched', async () => {
    await harness.store.open();
    harness.store.toggle('auth');
    harness.repository.failNext = new Error('boom');

    await harness.store.proposeRename('identity');

    expect(harness.store.pending()).toBeNull();
    expect(harness.notifier.notice()?.ref.key).toBe('errors.tagActionFailed');
  });

  it('keeps the selection when a confirmed action fails', async () => {
    await harness.store.open();
    harness.store.toggle('auth');
    await harness.store.proposeRename('identity');
    harness.repository.failNext = new Error('boom');

    expect(await harness.store.confirm()).toBe(false);
    expect(harness.store.selectedCount()).toBe(1);
    expect(harness.notifier.notice()?.ref.key).toBe('errors.tagActionFailed');
  });
});
