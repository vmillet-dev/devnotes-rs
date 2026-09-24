import { ComponentFixture, TestBed } from '@angular/core/testing';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { Folder } from '@core/model/folder.model';
import { Space } from '@core/model/space.model';
import { RAIL_WIDTH } from '@core/services/settings/app-settings.model';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { LibraryTreeComponent } from './library-tree.component';

const SPACES: readonly Space[] = [
  { id: 'sql', name: 'SQL', pinned: true },
  { id: 'rust', name: 'Rust', pinned: false },
];

function folder(id: string, spaceId: string, name: string): Folder {
  return { id, spaceId, name, colour: 'amber', createdAt: new Date('2026-01-01T10:00:00Z') };
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
      providers: [provideTranslocoTesting()],
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
    expect(names('[data-testid="folder-option"]')).toEqual(['Perf', 'Migrations', 'Async']);
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
      'Perf',
      'Migrations',
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
