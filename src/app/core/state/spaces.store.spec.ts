import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { IpcError } from '@core/ipc/ipc.error';
import { Space } from '@core/model/space.model';
import { FakeSpacesRepository } from '@testing/fake-spaces-repository';
import { provideAppTesting } from '@testing/testing.providers';
import { SpacesStore } from './spaces.store';

const SPACES: readonly Space[] = [
  { id: 'work', name: 'Work', pinned: false },
  { id: 'personal', name: 'Personal', pinned: false },
];

interface Harness {
  readonly store: SpacesStore;
  readonly repository: FakeSpacesRepository;
}

async function createStore(spaces: readonly Space[] = SPACES): Promise<Harness> {
  const repository = new FakeSpacesRepository(spaces);
  TestBed.configureTestingModule({ providers: [provideAppTesting({ spacesRepository: repository })] });

  const store = TestBed.inject(SpacesStore);
  await vi.waitFor(() => expect(store.isLoading()).toBe(false));

  return { store, repository };
}

describe('SpacesStore', () => {
  beforeEach(() => {
    TestBed.resetTestingModule();
    vi.restoreAllMocks();
    // Failures are reported through console.error on purpose.
    vi.spyOn(console, 'error').mockImplementation(() => undefined);
  });

  it('loads the spaces from the repository', async () => {
    const { store } = await createStore();

    expect(store.spaces()).toEqual(SPACES);
  });

  it('starts on "all spaces" rather than on an arbitrary one', async () => {
    const { store } = await createStore();

    expect(store.activeSpace()).toBeNull();
    expect(store.activeSpaceId()).toBeNull();
  });

  it('exposes the selected space', async () => {
    const { store } = await createStore();

    store.selectSpace('personal');

    expect(store.activeSpace()).toEqual(SPACES[1]);
    expect(store.activeSpaceId()).toBe('personal');
  });

  it('goes back to "all spaces" when selecting null', async () => {
    const { store } = await createStore();
    store.selectSpace('work');

    store.selectSpace(null);

    expect(store.activeSpace()).toBeNull();
  });

  it('falls back to "all spaces" when the selected id matches no space', async () => {
    // Otherwise a stale id would hide every note with no way to tell why.
    const { store } = await createStore();

    store.selectSpace('does-not-exist');

    expect(store.activeSpace()).toBeNull();
    expect(store.activeSpaceId()).toBeNull();
  });

  it('surfaces a load failure through the error notifier', async () => {
    // Nothing on screen goes blank when spaces fail to load.
    const repository = new FakeSpacesRepository(SPACES);
    repository.failNext = new Error('backend down');
    TestBed.configureTestingModule({ providers: [provideAppTesting({ spacesRepository: repository })] });
    const notifier = TestBed.inject(ErrorNotifier);

    TestBed.inject(SpacesStore);

    await vi.waitFor(() => expect(notifier.notice()?.ref.key).toBe('errors.spacesLoadFailed'));
    expect(notifier.notice()?.detail).toBe('backend down');
  });

  describe('createSpace', () => {
    it('appends the space returned by the repository and selects it', async () => {
      const { store } = await createStore();

      const created = await store.createSpace('Side project');

      expect(created?.name).toBe('Side project');
      expect(store.spaces().map((space) => space.name)).toEqual(['Work', 'Personal', 'Side project']);
      expect(store.activeSpaceId()).toBe(created?.id);
    });

    it('takes the id from the repository rather than generating one locally', async () => {
      const { store } = await createStore([]);

      const created = await store.createSpace('First');

      expect(created?.id).toBe('fake-space-1');
    });

    it('trims the submitted name', async () => {
      const { store } = await createStore([]);

      const created = await store.createSpace('  Padded  ');

      expect(created?.name).toBe('Padded');
    });

    it('ignores a blank name without calling the repository', async () => {
      const { store, repository } = await createStore();
      const create = vi.spyOn(repository, 'create');

      const created = await store.createSpace('   ');

      expect(created).toBeNull();
      expect(create).not.toHaveBeenCalled();
    });

    it('translates the backend refusal of an already-taken name', async () => {
      // Only the storage layer can see the real state of the database, and it answers
      // with a code, which is what makes a translated message possible.
      const { store, repository } = await createStore();
      const notifier = TestBed.inject(ErrorNotifier);
      repository.failNext = new IpcError('create_space', {
        code: 'duplicateSpaceName',
        params: { name: 'Work' },
        detail: 'Un espace nommé « Work » existe déjà',
      });

      const created = await store.createSpace('  wORK ');

      expect(created).toBeNull();
      expect(notifier.notice()?.ref.key).toBe('errors.spaceNameTaken');
      expect(notifier.notice()?.ref.params).toEqual({ name: 'Work' });
    });

    it('falls back to a generic message for any other backend failure', async () => {
      const { store, repository } = await createStore();
      const notifier = TestBed.inject(ErrorNotifier);
      repository.failNext = new IpcError('create_space', {
        code: 'storage',
        params: {},
        detail: 'Erreur de stockage : disk I/O error',
      });

      await store.createSpace('Work');

      expect(notifier.notice()?.ref.key).toBe('errors.spaceCreateFailed');
      expect(notifier.notice()?.detail).toContain('disk I/O error');
    });

    it('notifies and adds nothing when creation fails', async () => {
      const { store, repository } = await createStore();
      const notifier = TestBed.inject(ErrorNotifier);
      repository.failNext = new Error('read-only');

      const created = await store.createSpace('Doomed');

      expect(created).toBeNull();
      expect(store.spaces()).toEqual(SPACES);
      expect(notifier.notice()?.ref.key).toBe('errors.spaceCreateFailed');
    });
  });

  describe('renameSpace', () => {
    it('adopts the space the repository returned', async () => {
      const { store } = await createStore();

      const renamed = await store.renameSpace('work', 'Boulot');

      expect(renamed).toBe(true);
      expect(store.spaces().find((space) => space.id === 'work')?.name).toBe('Boulot');
    });

    it('trims the name before sending it', async () => {
      const { store, repository } = await createStore();
      const rename = vi.spyOn(repository, 'rename');

      await store.renameSpace('work', '  Boulot  ');

      expect(rename).toHaveBeenCalledWith('work', { name: 'Boulot' });
    });

    it('writes nothing when the name is unchanged or blank', async () => {
      const { store, repository } = await createStore();
      const rename = vi.spyOn(repository, 'rename');

      expect(await store.renameSpace('work', 'Work')).toBe(false);
      expect(await store.renameSpace('work', '   ')).toBe(false);

      expect(rename).not.toHaveBeenCalled();
    });

    it('reports a name already taken with the name the user typed', async () => {
      const { store, repository } = await createStore();
      const notifier = TestBed.inject(ErrorNotifier);
      repository.failNext = new IpcError('rename_space', {
        code: 'duplicateSpaceName',
        params: { name: 'Personal' },
        detail: 'Un espace nommé « Personal » existe déjà',
      });

      const renamed = await store.renameSpace('work', 'Personal');

      expect(renamed).toBe(false);
      expect(notifier.notice()?.ref.key).toBe('errors.spaceNameTaken');
      expect(notifier.notice()?.ref.params).toEqual({ name: 'Personal' });
      // The list must not have drifted from the backend.
      expect(store.spaces()).toEqual(SPACES);
    });
  });

  describe('togglePinned', () => {
    it('sends the opposite of what the space carries', async () => {
      const { store, repository } = await createStore();
      const setPinned = vi.spyOn(repository, 'setPinned');

      await store.togglePinned('work');

      expect(setPinned).toHaveBeenCalledWith('work', true);
    });

    /** Reloaded rather than patched in place: pinning changes the order, which is the back end's. */
    it('takes the new order from the backend rather than keeping its own', async () => {
      const { store } = await createStore();
      expect(store.spaces().map((space) => space.id)).toEqual(['work', 'personal']);

      await store.togglePinned('personal');

      // Waited for: the reload is a round trip, so the new order arrives after the call.
      await vi.waitFor(() => expect(store.spaces().map((space) => space.id)).toEqual(['personal', 'work']));
      expect(store.spaces()[0].pinned).toBe(true);
    });

    it('writes nothing for a space it does not hold', async () => {
      const { store, repository } = await createStore();
      const setPinned = vi.spyOn(repository, 'setPinned');

      expect(await store.togglePinned('missing')).toBe(false);
      expect(setPinned).not.toHaveBeenCalled();
    });

    it('leaves the list alone when the write fails, and says so', async () => {
      const { store, repository } = await createStore();
      const notifier = TestBed.inject(ErrorNotifier);
      repository.failNext = new Error('disk full');

      expect(await store.togglePinned('work')).toBe(false);
      expect(notifier.notice()?.ref.key).toBe('errors.spacePinFailed');
      expect(store.spaces()).toEqual(SPACES);
    });
  });

  describe('deleteSpace', () => {
    it('drops the space and activates the one that took its notes', async () => {
      const { store } = await createStore();

      const deleted = await store.deleteSpace('work', 'personal');

      expect(deleted).toBe(true);
      expect(store.spaces().map((space) => space.id)).toEqual(['personal']);
      // Falling back to "all spaces" would lose sight of where the notes went.
      expect(store.activeSpaceId()).toBe('personal');
    });

    it('refuses to make a space its own refuge', async () => {
      const { store, repository } = await createStore();
      const remove = vi.spyOn(repository, 'delete');

      const deleted = await store.deleteSpace('work', 'work');

      // The schema cascades on delete: the notes would be taken back out one statement
      // after being "moved".
      expect(deleted).toBe(false);
      expect(remove).not.toHaveBeenCalled();
      expect(store.spaces()).toEqual(SPACES);
    });

    it('refuses a refuge that is not a known space', async () => {
      const { store, repository } = await createStore();
      const remove = vi.spyOn(repository, 'delete');

      expect(await store.deleteSpace('work', 'ghost')).toBe(false);

      expect(remove).not.toHaveBeenCalled();
    });

    it('keeps the space listed when the backend refuses', async () => {
      const { store, repository } = await createStore();
      const notifier = TestBed.inject(ErrorNotifier);
      repository.failNext = new Error('read-only');

      const deleted = await store.deleteSpace('work', 'personal');

      expect(deleted).toBe(false);
      expect(store.spaces()).toEqual(SPACES);
      expect(notifier.notice()?.ref.key).toBe('errors.spaceDeleteFailed');
    });
  });
});
