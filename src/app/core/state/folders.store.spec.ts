import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { Folder } from '@core/model/folder.model';
import { Space } from '@core/model/space.model';
import { FakeFoldersRepository } from '@testing/fake-folders-repository';
import { FakeSpacesRepository } from '@testing/fake-spaces-repository';
import { provideAppTesting } from '@testing/testing.providers';
import { FoldersStore } from './folders.store';
import { NotesRevision } from './notes-revision';
import { SpacesStore } from './spaces.store';

const SPACES: readonly Space[] = [
  { id: 'sql', name: 'SQL', pinned: false },
  { id: 'veille', name: 'Veille', pinned: false },
];

function folder(id: string, spaceId: string, name: string): Folder {
  return { id, spaceId, name, colour: 'blue', createdAt: new Date('2026-01-01T10:00:00Z') };
}

const FOLDERS: readonly Folder[] = [
  folder('perf', 'sql', 'Perf'),
  folder('migrations', 'sql', 'Migrations'),
  folder('liens', 'veille', 'Liens'),
];

interface Harness {
  readonly store: FoldersStore;
  readonly spaces: SpacesStore;
  readonly repository: FakeFoldersRepository;
  readonly revision: NotesRevision;
}

async function createStore(folders: readonly Folder[] = FOLDERS): Promise<Harness> {
  const repository = new FakeFoldersRepository(folders);
  TestBed.configureTestingModule({
    providers: [
      provideAppTesting({
        foldersRepository: repository,
        spacesRepository: new FakeSpacesRepository(SPACES),
      }),
    ],
  });

  const spaces = TestBed.inject(SpacesStore);
  await vi.waitFor(() => expect(spaces.isLoading()).toBe(false));

  const store = TestBed.inject(FoldersStore);
  await vi.waitFor(() => expect(store.isLoading()).toBe(false));

  return { store, spaces, repository, revision: TestBed.inject(NotesRevision) };
}

/** The store follows the space, so a spec about one folder has to settle there first. */
async function inSpace(harness: Harness, spaceId: string): Promise<void> {
  harness.spaces.selectSpace(spaceId);
  await vi.waitFor(() => expect(harness.store.isLoading()).toBe(false));
}

describe('FoldersStore', () => {
  beforeEach(() => {
    TestBed.resetTestingModule();
    vi.restoreAllMocks();
    // Failures are reported through console.error on purpose.
    vi.spyOn(console, 'error').mockImplementation(() => undefined);
  });

  it('starts on "every folder" rather than on an arbitrary one', async () => {
    const { store } = await createStore();

    expect(store.activeFolderId()).toBeNull();
    expect(store.activeFolder()).toBeNull();
  });

  it('narrows to the active space, so another space’s folders never show', async () => {
    const harness = await createStore();

    await inSpace(harness, 'sql');

    expect(harness.store.folders().map((each) => each.id)).toEqual(['perf', 'migrations']);
  });

  it('serves every folder while the user is on all spaces', async () => {
    const { store } = await createStore();

    expect(store.folders()).toHaveLength(3);
  });

  it('resolves the selected folder', async () => {
    const harness = await createStore();
    await inSpace(harness, 'sql');

    harness.store.selectFolder('perf');

    expect(harness.store.activeFolder()?.name).toBe('Perf');
  });

  /** Otherwise switching space would leave the canvas narrowed to nothing at all. */
  it('falls back to every folder when the selected one is not in this space', async () => {
    const harness = await createStore();
    await inSpace(harness, 'sql');
    harness.store.selectFolder('perf');

    await inSpace(harness, 'veille');

    expect(harness.store.activeFolderId()).toBeNull();
  });

  it('creates a folder in the active space and adopts what persistence returned', async () => {
    const harness = await createStore([]);
    await inSpace(harness, 'sql');

    const created = await harness.store.createFolder('  Perf  ');

    expect(created?.name).toBe('Perf');
    expect(created?.spaceId).toBe('sql');
    expect(harness.store.folders()).toHaveLength(1);
  });

  /**
   * The rail shows it at once from the adopted list; the board reads its zones through
   * a query of its own and would not draw the new one until something else reloaded it.
   */
  it('bumps the revision on a creation, so the board draws the zone', async () => {
    const harness = await createStore([]);
    await inSpace(harness, 'sql');
    const before = harness.revision.current();

    await harness.store.createFolder('Perf');

    expect(harness.revision.current()).toBeGreaterThan(before);
  });

  /** A folder belongs to a space, so there is nothing to create it in. */
  it('refuses to create a folder while no space is chosen', async () => {
    const { store } = await createStore([]);

    expect(await store.createFolder('Perf')).toBeNull();
    expect(store.folders()).toHaveLength(0);
  });

  it('refuses a blank name without a round trip', async () => {
    const harness = await createStore([]);
    await inSpace(harness, 'sql');
    const create = vi.spyOn(harness.repository, 'create');

    expect(await harness.store.createFolder('   ')).toBeNull();
    expect(create).not.toHaveBeenCalled();
  });

  it('renames a folder', async () => {
    const harness = await createStore();
    await inSpace(harness, 'sql');

    await harness.store.renameFolder('perf', 'Performance');

    expect(harness.store.folders().find((each) => each.id === 'perf')?.name).toBe('Performance');
  });

  /** The chips on the cards carry the name, and the back end is what resolved them. */
  it('bumps the revision on a rename, so the cards redraw their chip', async () => {
    const harness = await createStore();
    await inSpace(harness, 'sql');
    const before = harness.revision.current();

    await harness.store.renameFolder('perf', 'Performance');

    expect(harness.revision.current()).toBeGreaterThan(before);
  });

  it('leaves a rename to the same name alone', async () => {
    const harness = await createStore();
    await inSpace(harness, 'sql');
    const rename = vi.spyOn(harness.repository, 'rename');

    expect(await harness.store.renameFolder('perf', 'Perf')).toBe(false);
    expect(rename).not.toHaveBeenCalled();
  });

  it('recolours a folder', async () => {
    const harness = await createStore();
    await inSpace(harness, 'sql');

    await harness.store.recolourFolder('perf', 'red');

    expect(harness.store.folders().find((each) => each.id === 'perf')?.colour).toBe('red');
  });

  it('deletes a folder without asking for a refuge', async () => {
    const harness = await createStore();
    await inSpace(harness, 'sql');

    await harness.store.deleteFolder('perf');

    expect(harness.store.folders().map((each) => each.id)).toEqual(['migrations']);
  });

  /** The canvas was narrowed to it; leaving it selected would empty the screen. */
  it('falls back to every folder after deleting the selected one', async () => {
    const harness = await createStore();
    await inSpace(harness, 'sql');
    harness.store.selectFolder('perf');

    await harness.store.deleteFolder('perf');

    expect(harness.store.activeFolderId()).toBeNull();
  });

  it('reports a failed creation and keeps the list as it was', async () => {
    const harness = await createStore([]);
    await inSpace(harness, 'sql');
    harness.repository.failNext = new Error('disk full');

    expect(await harness.store.createFolder('Perf')).toBeNull();
    expect(harness.store.folders()).toHaveLength(0);
  });
});
