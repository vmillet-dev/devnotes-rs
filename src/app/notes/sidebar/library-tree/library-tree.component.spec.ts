import { ComponentFixture, TestBed } from '@angular/core/testing';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { Folder } from '@core/model/folder.model';
import { Space } from '@core/model/space.model';
import { RAIL_WIDTH } from '@core/services/settings/app-settings.model';
import { FoldersStore } from '@core/state/folders.store';
import { UndoStore } from '@core/state/undo.store';
import { FakeFoldersRepository } from '@testing/fake-folders-repository';
import { provideAppTesting } from '@testing/testing.providers';
import { LibraryTreeComponent } from './library-tree.component';

const SPACES: readonly Space[] = [
  { id: 'sql', name: 'SQL', pinned: true },
  { id: 'rust', name: 'Rust', pinned: false },
];

function folder(id: string, spaceId: string, name: string): Folder {
  return { id, spaceId, name, colour: 'amber', createdAt: new Date('2026-01-01T10:00:00Z'), parentId: null };
}

const FOLDERS: readonly Folder[] = [
  folder('perf', 'sql', 'Perf'),
  folder('migr', 'sql', 'Migrations'),
  folder('async', 'rust', 'Async'),
];

describe('LibraryTreeComponent', () => {
  let fixture: ComponentFixture<LibraryTreeComponent>;

  function root(): HTMLElement {
    return fixture.nativeElement;
  }

  function each(selector: string): HTMLElement[] {
    return [...root().querySelectorAll<HTMLElement>(selector)];
  }

  function names(selector: string): (string | undefined)[] {
    return each(selector).map((element) => element.textContent?.trim());
  }

  async function click(selector: string): Promise<void> {
    root().querySelector<HTMLElement>(selector)?.click();
    await fixture.whenStable();
  }

  beforeEach(async () => {
    TestBed.configureTestingModule({
      imports: [LibraryTreeComponent],
      providers: [provideAppTesting({ spaces: SPACES, folders: FOLDERS })],
    });
    fixture = TestBed.createComponent(LibraryTreeComponent);
    fixture.componentRef.setInput('spaces', SPACES);
    fixture.componentRef.setInput('folders', FOLDERS);
    fixture.componentRef.setInput('activeSpaceId', 'sql');
    fixture.componentRef.setInput('activeFolderId', null);
    // jsdom only tracks `document.activeElement` for attached elements.
    document.body.appendChild(fixture.nativeElement);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  afterEach(() => {
    fixture.nativeElement.remove();
  });

  it('draws every space with its own folders under it', () => {
    const spaces = names('[data-testid="space-option"]');

    expect(spaces).toHaveLength(2);
    expect(spaces[0]).toContain('SQL');
    // The 📌 is decorative, so the row carries a text twin next to it.
    expect(spaces[0]).toContain('Espace épinglé');
    // By name: siblings have no order of their own.
    expect(names('[data-testid="folder-option"]')).toEqual(['Migrations', 'Perf', 'Async']);
  });

  /**
   * The name sits in a box of its own rather than loose in the row: `text-overflow` is
   * a block container's property and the row is a flex one, which ignores it — a name too
   * long for the rail was cut mid-letter instead of ellipsised.
   */
  it('gives every row name a box the ellipsis can apply to', () => {
    expect(names('.node-label')).toEqual([
      'Tous les espaces',
      'SQL',
      'Migrations',
      'Perf',
      'Nouveau dossier',
      'Rust',
      'Async',
    ]);
  });

  it('opens on the whole library rather than on what was expanded before', () => {
    expect(each('[data-testid="space-twisty"]').map((t) => t.getAttribute('aria-expanded'))).toEqual([
      'true',
      'true',
    ]);
  });

  it('collapses a space without changing what is active', async () => {
    await click('[data-testid="space-twisty"][data-space-id="sql"]');

    expect(names('[data-testid="folder-option"]')).toEqual(['Async']);
  });

  it('marks the active space and the open folder', async () => {
    fixture.componentRef.setInput('activeFolderId', 'perf');
    await fixture.whenStable();

    expect(
      root().querySelector('[data-testid="space-option"][data-space-id="sql"]')?.className,
    ).not.toContain('active');
    expect(root().querySelector('[data-testid="folder-option"][data-folder-id="perf"]')?.className).toContain(
      'active',
    );
  });

  it('answers a chosen space, and "all spaces" with null', async () => {
    const chosen: (string | null)[] = [];
    fixture.componentInstance.spaceChanged.subscribe((id) => chosen.push(id));

    await click('[data-testid="space-option"][data-space-id="rust"]');
    await click('[data-testid="space-option-all"]');

    expect(chosen).toEqual(['rust', null]);
  });

  it('answers the whole folder, because opening one means switching space too', async () => {
    const opened: Folder[] = [];
    fixture.componentInstance.folderOpened.subscribe((folder) => opened.push(folder));

    await click('[data-testid="folder-option"][data-folder-id="async"]');

    expect(opened).toEqual([FOLDERS[2]]);
  });

  it('creates a space from the form the ＋ opens', async () => {
    const created: string[] = [];
    fixture.componentInstance.spaceCreated.subscribe((name) => created.push(name));

    await click('[data-testid="space-create-open"]');
    const input = root().querySelector<HTMLInputElement>('[data-testid="space-create-input"]');
    // The field takes the focus: a form that opens under the pointer is still typed into.
    expect(document.activeElement).toBe(input);

    input!.value = 'Ops';
    root().querySelector('form')?.dispatchEvent(new Event('submit'));
    await fixture.whenStable();

    expect(created).toEqual(['Ops']);
  });

  /** Under the active space alone: a folder is created in the space one is in. */
  it('offers to create a folder in the active space only', async () => {
    expect(each('[data-testid="folder-create-open"]')).toHaveLength(1);

    fixture.componentRef.setInput('activeSpaceId', null);
    await fixture.whenStable();

    expect(each('[data-testid="folder-create-open"]')).toHaveLength(0);
  });

  it('opens the space panel from its ⋯, and closes it on a second click', async () => {
    await click('[data-testid="space-edit"][data-space-id="sql"]');
    expect(root().querySelector('app-space-editor')).not.toBeNull();

    await click('[data-testid="space-edit"][data-space-id="sql"]');
    expect(root().querySelector('app-space-editor')).toBeNull();
  });

  /** One panel at a time: two open forms in a rail this narrow read as one. */
  it('closes the space panel when a folder panel opens', async () => {
    await click('[data-testid="space-edit"][data-space-id="sql"]');
    await click('[data-testid="folder-edit"][data-folder-id="perf"]');

    expect(root().querySelector('app-space-editor')).toBeNull();
    expect(root().querySelector('app-folder-editor')).not.toBeNull();
  });

  /** The arrow keys are not optional: the drag is pointer events, which no keyboard has. */
  it('nudges its own width from the edge, and stops at the bounds', async () => {
    const widths: number[] = [];
    fixture.componentInstance.widthChanged.subscribe((width) => widths.push(width));
    const edge = root().querySelector<HTMLElement>('[data-testid="library-rail-edge"]')!;

    edge.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true }));
    await fixture.whenStable();
    expect(widths).toEqual([RAIL_WIDTH.default + 16]);

    fixture.componentRef.setInput('width', RAIL_WIDTH.min);
    await fixture.whenStable();
    edge.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowLeft', bubbles: true }));
    await fixture.whenStable();

    // Already at the minimum: clamped to the same number, so nothing is emitted.
    expect(widths).toEqual([RAIL_WIDTH.default + 16]);
  });

  it('offers every other space as a refuge when a space is being deleted', async () => {
    await click('[data-testid="space-edit"][data-space-id="sql"]');

    root().querySelector<HTMLElement>('[data-testid="choice-space-move-target"]')!.click();
    await fixture.whenStable();

    const targets = [
      ...root().querySelectorAll('[data-testid="choice-panel-space-move-target"] [data-option-id]'),
    ];
    expect(targets.map((option) => option.getAttribute('data-option-id'))).toEqual(['rust']);
  });
});

/** Folders inside folders: drawn under their parent, moved by a drag or by `Alt` and the arrows. */
describe('LibraryTreeComponent, nested', () => {
  // SQL: Base › (Perf › Index), Archives; Rust: Async.
  const NESTED: readonly Folder[] = [
    folder('base', 'sql', 'Base'),
    { ...folder('perf', 'sql', 'Perf'), parentId: 'base' },
    { ...folder('index', 'sql', 'Index'), parentId: 'perf' },
    folder('archives', 'sql', 'Archives'),
    folder('async', 'rust', 'Async'),
  ];

  let fixture: ComponentFixture<LibraryTreeComponent>;
  let repository: FakeFoldersRepository;

  const el = <T extends HTMLElement = HTMLElement>(selector: string): T | null =>
    (fixture.nativeElement as HTMLElement).querySelector<T>(selector);
  const name = (id: string) => el(`[data-testid="folder-option"][data-folder-id="${id}"]`)!;
  const row = (id: string) => el(`[data-rail-row][data-id="${id}"]`)!;
  const said = () => el('[data-testid="library-rail-moved"]')?.textContent?.trim();

  beforeEach(async () => {
    repository = new FakeFoldersRepository(NESTED);
    TestBed.configureTestingModule({
      providers: [provideAppTesting({ spaces: SPACES, foldersRepository: repository })],
    });
    await vi.waitFor(() => expect(TestBed.inject(FoldersStore).allFolders()).toHaveLength(NESTED.length));
    fixture = TestBed.createComponent(LibraryTreeComponent);
    fixture.componentRef.setInput('spaces', SPACES);
    fixture.componentRef.setInput('folders', NESTED);
    fixture.componentRef.setInput('activeSpaceId', 'sql');
    fixture.componentRef.setInput('activeFolderId', null);
    document.body.appendChild(fixture.nativeElement);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  afterEach(() => {
    fixture.nativeElement.remove();
  });

  it('draws a folder under its parent, siblings by name, with a twisty where it holds folders', () => {
    const names = [
      ...(fixture.nativeElement as HTMLElement).querySelectorAll('[data-testid="folder-option"]'),
    ].map((each) => each.getAttribute('data-folder-id'));

    expect(names).toEqual(['archives', 'base', 'perf', 'index', 'async']);
    expect(row('base').querySelector('.folders [data-id="perf"]')).not.toBeNull();
    expect(el('[data-testid="folder-twisty"][data-folder-id="base"]')).not.toBeNull();
    expect(el('[data-testid="folder-twisty"][data-folder-id="archives"]')).toBeNull();
  });

  it('folds a folder away, and opens it again', async () => {
    el('[data-testid="folder-twisty"][data-folder-id="perf"]')!.click();
    await fixture.whenStable();
    expect(el('[data-id="index"]')).toBeNull();

    el('[data-testid="folder-twisty"][data-folder-id="perf"]')!.click();
    await fixture.whenStable();
    expect(el('[data-id="index"]')).not.toBeNull();
  });

  it('opens the folder a sub-folder is made in from its ⋯, and marks the new row', async () => {
    el('[data-testid="folder-twisty"][data-folder-id="perf"]')!.click();
    await fixture.whenStable();
    el('[data-testid="folder-edit"][data-folder-id="perf"]')!.click();
    await fixture.whenStable();
    el('[data-testid="folder-subfolder-open"]')!.click();
    await fixture.whenStable();
    const input = el<HTMLInputElement>('[data-testid="folder-subfolder-input"]')!;
    input.value = 'Plans';
    input.form!.dispatchEvent(new Event('submit'));

    const store = TestBed.inject(FoldersStore);
    await vi.waitFor(() => expect(store.allFolders().some((each) => each.name === 'Plans')).toBe(true));
    fixture.componentRef.setInput('folders', store.allFolders());
    await fixture.whenStable();

    const made = store.allFolders().find((each) => each.name === 'Plans')!;
    expect(made).toMatchObject({ spaceId: 'sql', parentId: 'perf' });
    expect(el('app-folder-editor')).toBeNull();
    expect(row('perf').querySelector(`.folders [data-id="${made.id}"]`)).not.toBeNull();
    expect(row(made.id).getAttribute('data-moved')).toBe('row');
    expect(row('perf').getAttribute('data-moved')).toBe('parent');
  });

  it('moves a folder into the one above it with Alt+→, says so and offers it back', async () => {
    name('base').dispatchEvent(
      new KeyboardEvent('keydown', { key: 'ArrowRight', altKey: true, bubbles: true }),
    );

    await vi.waitFor(() => expect(said()).toBe('« Base » est maintenant dans Archives.'));
    expect((await repository.loadAll('sql')).find((each) => each.id === 'base')?.parentId).toBe('archives');
    expect(TestBed.inject(UndoStore).last()).toMatchObject({
      kind: 'folderMove',
      name: 'Base',
      into: 'Archives',
    });
  });

  it('takes a folder up a level with Alt+←, and refuses at the top', async () => {
    name('perf').dispatchEvent(
      new KeyboardEvent('keydown', { key: 'ArrowLeft', altKey: true, bubbles: true }),
    );
    await vi.waitFor(() => expect(said()).toBe('« Perf » est maintenant en haut de SQL.'));
    expect((await repository.loadAll('sql')).find((each) => each.id === 'perf')?.parentId).toBeNull();

    name('archives').dispatchEvent(
      new KeyboardEvent('keydown', { key: 'ArrowLeft', altKey: true, bubbles: true }),
    );
    await vi.waitFor(() => expect(said()).toBe('« Archives » est déjà en haut de son espace.'));
    name('archives').dispatchEvent(
      new KeyboardEvent('keydown', { key: 'ArrowRight', altKey: true, bubbles: true }),
    );
    await vi.waitFor(() => expect(said()).toBe('Aucun dossier au-dessus de « Archives » pour l’y ranger.'));
  });

  /** jsdom has no layout: the row under the pointer is said by hand. */
  function pointAt(target: HTMLElement): void {
    document.elementFromPoint = () => target;
  }

  function drag(from: HTMLElement): void {
    from.dispatchEvent(
      new PointerEvent('pointerdown', { button: 0, pointerId: 1, clientX: 10, clientY: 10, bubbles: true }),
    );
    from.dispatchEvent(
      new PointerEvent('pointermove', { pointerId: 1, clientX: 10, clientY: 60, bubbles: true }),
    );
  }

  it('moves a folder dragged onto another, and the click that ends the drag opens nothing', async () => {
    const opened: string[] = [];
    fixture.componentInstance.folderOpened.subscribe((each) => opened.push(each.id));
    pointAt(row('archives'));

    drag(name('perf'));
    await fixture.whenStable();
    expect(row('archives').classList).toContain('drop-inside');
    expect(row('perf').classList).toContain('dragging');

    name('perf').dispatchEvent(
      new PointerEvent('pointerup', { pointerId: 1, clientX: 10, clientY: 60, bubbles: true }),
    );
    name('perf').click();

    await vi.waitFor(async () =>
      expect((await repository.loadAll('sql')).find((each) => each.id === 'perf')?.parentId).toBe('archives'),
    );
    expect(opened).toEqual([]);
  });

  it('refuses a folder dragged into what it holds, in red, and says it', async () => {
    pointAt(row('index'));

    drag(name('base'));
    await fixture.whenStable();
    expect(row('index').classList).toContain('drop-refused');

    name('base').dispatchEvent(
      new PointerEvent('pointerup', { pointerId: 1, clientX: 10, clientY: 60, bubbles: true }),
    );
    await vi.waitFor(() => expect(said()).toBe('« Base » ne peut pas aller là.'));
    expect((await repository.loadAll('sql')).find((each) => each.id === 'base')?.parentId).toBeNull();
  });

  it('brings a folder dropped on its space row back to the top, and refuses another space', async () => {
    pointAt(row('rust'));
    drag(name('perf'));
    await fixture.whenStable();
    expect(row('rust').classList).toContain('drop-refused');
    name('perf').dispatchEvent(new PointerEvent('pointercancel', { pointerId: 1, bubbles: true }));

    pointAt(row('sql'));
    drag(name('perf'));
    name('perf').dispatchEvent(
      new PointerEvent('pointerup', { pointerId: 1, clientX: 10, clientY: 60, bubbles: true }),
    );
    await vi.waitFor(async () =>
      expect((await repository.loadAll('sql')).find((each) => each.id === 'perf')?.parentId).toBeNull(),
    );
  });

  it('lights the row a card dragged on the board would be filed into', async () => {
    fixture.componentRef.setInput('cardTarget', 'index');
    await fixture.whenStable();

    expect(row('index').classList).toContain('drop-inside');
  });
});
