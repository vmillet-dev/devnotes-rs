import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { Folder } from '@core/model/folder.model';
import { Space } from '@core/model/space.model';
import { LibraryPreferencesService } from '@core/services/preferences/library-preferences.service';
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
  return { id, spaceId, name, colour: 'blue', createdAt: new Date('2026-01-01T10:00:00Z'), parentId: null };
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

  /** « Nouveau dossier » from the palette: the name is asked, then the folder made where the notes are. */
  it('asks a name in the space open, creates the folder named, and asks nothing across every space', async () => {
    const harness = await createStore([]);
    harness.store.askName();
    expect(harness.store.naming()).toBe(false);

    await inSpace(harness, 'sql');
    harness.store.askName();
    expect(harness.store.naming()).toBe(true);
    await harness.store.createNamed('Perf');

    expect(harness.store.naming()).toBe(false);
    expect(harness.store.folders().map((folder) => folder.name)).toEqual(['Perf']);

    harness.store.askName();
    harness.store.dismissNaming();
    expect(harness.store.naming()).toBe(false);
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
  describe('the folder the library was left in', () => {
    it('remembers the folder opened, and its closing', async () => {
      const { store, spaces } = await createStore();
      const preferences = TestBed.inject(LibraryPreferencesService);
      spaces.selectSpace('sql');

      store.selectFolder('perf');
      expect(preferences.read('devnotes.notes.activeFolder')).toBe('perf');

      store.selectFolder(null);
      expect(preferences.read('devnotes.notes.activeFolder')).toBe('');
    });

    it('opens it again with its space, and not in another space', async () => {
      const { store, spaces } = await createStore();
      TestBed.inject(LibraryPreferencesService).write('devnotes.notes.activeFolder', 'perf');

      spaces.selectSpace('sql');
      store.restoreRemembered();
      expect(store.activeFolderId()).toBe('perf');

      spaces.selectSpace('veille');
      expect(store.activeFolderId()).toBeNull();
    });
  });
});

describe('FoldersStore, nested', () => {
  // SQL › Perf › Index, and Migrations beside Perf.
  const NESTED: readonly Folder[] = [
    folder('sql-f', 'sql', 'SQL'),
    { ...folder('perf', 'sql', 'Perf'), parentId: 'sql-f' },
    { ...folder('index', 'sql', 'Index'), parentId: 'perf' },
    { ...folder('migrations', 'sql', 'Migrations'), parentId: 'sql-f' },
  ];

  it('says the path of the folder open and whether it holds folders', async () => {
    const harness = await createStore(NESTED);
    await inSpace(harness, 'sql');

    harness.store.selectFolder('perf');

    expect(harness.store.activePath().map((each) => each.name)).toEqual(['SQL', 'Perf']);
    expect(harness.store.activeHasChildren()).toBe(true);
    harness.store.selectFolder('index');
    expect(harness.store.activeHasChildren()).toBe(false);
  });

  it('goes up a level at a time, then out', async () => {
    const harness = await createStore(NESTED);
    await inSpace(harness, 'sql');
    harness.store.selectFolder('index');

    expect(harness.store.goUp()).toBe(true);
    expect(harness.store.activeFolderId()).toBe('perf');
    harness.store.goUp();
    harness.store.goUp();
    expect(harness.store.activeFolderId()).toBeNull();
    expect(harness.store.goUp()).toBe(false);
  });

  it('lists a level of one space by name', async () => {
    const harness = await createStore(NESTED);

    expect(harness.store.childrenOf('sql-f', 'sql').map((each) => each.name)).toEqual(['Migrations', 'Perf']);
  });

  it('creates a folder under another', async () => {
    const harness = await createStore(NESTED);
    await inSpace(harness, 'sql');

    const created = await harness.store.createFolder('Requêtes', 'perf');

    expect(created?.parentId).toBe('perf');
  });

  it('creates a sub-folder in its parent space, whichever space is open', async () => {
    const harness = await createStore(NESTED);
    await inSpace(harness, 'veille');

    const created = await harness.store.createFolder('Requêtes', 'perf');

    expect(created).toMatchObject({ spaceId: 'sql', parentId: 'perf' });
  });

  it('creates nothing under a folder it does not know', async () => {
    const harness = await createStore(NESTED);
    await inSpace(harness, 'sql');

    expect(await harness.store.createFolder('Requêtes', 'gone')).toBeNull();
  });

  it('moves a folder, adopts its new parent and answers where it was', async () => {
    const harness = await createStore(NESTED);
    await inSpace(harness, 'sql');
    const before = harness.revision.current();

    const was = await harness.store.moveFolder('perf', 'migrations');

    expect(was).toEqual({ folderId: 'perf', parentId: 'sql-f', frame: null });
    expect(harness.store.allFolders().find((each) => each.id === 'perf')?.parentId).toBe('migrations');
    expect(harness.revision.current()).toBeGreaterThan(before);
  });

  it('says so when a move fails, and changes nothing', async () => {
    const harness = await createStore(NESTED);
    harness.repository.failNext = new Error('refused');

    expect(await harness.store.moveFolder('perf', null)).toBeNull();
    expect(harness.store.allFolders().find((each) => each.id === 'perf')?.parentId).toBe('sql-f');
  });

  it('hands what a deleted folder held to its parent, and opens the parent', async () => {
    const harness = await createStore(NESTED);
    await inSpace(harness, 'sql');
    harness.store.selectFolder('perf');

    await harness.store.deleteFolder('perf');

    expect(harness.store.activeFolderId()).toBe('sql-f');
    expect(harness.store.allFolders().find((each) => each.id === 'index')?.parentId).toBe('sql-f');
  });
});
