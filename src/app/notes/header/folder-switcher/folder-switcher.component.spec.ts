import { ComponentFixture, TestBed } from '@angular/core/testing';
import { By } from '@angular/platform-browser';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { Folder } from '@core/model/folder.model';
import { FoldersStore } from '@core/state/folders.store';
import { RecordingFolderActions } from '@testing/recording-actions';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { FolderSwitcherComponent } from './folder-switcher.component';

function folder(id: string, name: string, colour: Folder['colour']): Folder {
  return { id, spaceId: 'sql', name, colour, createdAt: new Date('2026-01-01T10:00:00Z') };
}

const FOLDERS: readonly Folder[] = [folder('perf', 'Perf', 'amber'), folder('migr', 'Migrations', 'blue')];

describe('FolderSwitcherComponent', () => {
  let fixture: ComponentFixture<FolderSwitcherComponent>;
  let actions: RecordingFolderActions;

  function root(): HTMLElement {
    return fixture.nativeElement;
  }

  function trigger(): HTMLButtonElement {
    return root().querySelector('.folder-switch') as HTMLButtonElement;
  }

  /** Every menu entry, in DOM order: "all folders", the folders, then "new folder". */
  function options(): HTMLButtonElement[] {
    return [...root().querySelectorAll<HTMLButtonElement>('.folder-option')];
  }

  async function open(): Promise<void> {
    trigger().click();
    await fixture.whenStable();
  }

  async function click(selector: string): Promise<void> {
    root().querySelector<HTMLElement>(selector)?.click();
    await fixture.whenStable();
  }

  beforeEach(async () => {
    actions = new RecordingFolderActions();
    TestBed.configureTestingModule({
      imports: [FolderSwitcherComponent],
      providers: [provideTranslocoTesting(), { provide: FoldersStore, useValue: actions }],
    });
    fixture = TestBed.createComponent(FolderSwitcherComponent);
    fixture.componentRef.setInput('folders', FOLDERS);
    fixture.componentRef.setInput('activeFolder', null);
    // jsdom only tracks `document.activeElement` for attached elements.
    document.body.appendChild(fixture.nativeElement);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  afterEach(() => {
    fixture.nativeElement.remove();
  });

  it('reads "all folders" until one narrows the canvas', () => {
    expect(trigger().textContent).toContain('Tous les dossiers');
    expect(fixture.debugElement.query(By.css('.folder-dropdown'))).toBeNull();
  });

  it('shows the narrowed folder with its swatch', async () => {
    fixture.componentRef.setInput('activeFolder', FOLDERS[0]);
    await fixture.whenStable();

    expect(trigger().textContent).toContain('Perf');
    expect(trigger().querySelector('.folder-swatch')?.className).toContain('is-amber');
  });

  it('lists every folder, framed by "all folders" and "new folder"', async () => {
    await open();

    expect(options().map((option) => option.textContent?.trim())).toEqual([
      'Tous les dossiers',
      'Perf',
      'Migrations',
      '＋ Nouveau dossier',
    ]);
  });

  it('emits null when "all folders" is chosen', async () => {
    fixture.componentRef.setInput('activeFolder', FOLDERS[0]);
    const seen: (string | null)[] = [];
    fixture.componentInstance.folderChanged.subscribe((id) => seen.push(id));

    await open();
    await click('[data-testid="folder-option-all"]');

    expect(seen).toEqual([null]);
  });

  it('emits the folder id when one is chosen', async () => {
    const seen: (string | null)[] = [];
    fixture.componentInstance.folderChanged.subscribe((id) => seen.push(id));

    await open();
    await click('[data-testid="folder-option"]');

    expect(seen).toEqual(['perf']);
  });

  it('creates a folder from the name typed in the panel', async () => {
    const seen: string[] = [];
    fixture.componentInstance.folderCreated.subscribe((name) => seen.push(name));

    await open();
    await click('[data-testid="folder-create-open"]');
    const input = root().querySelector<HTMLInputElement>('.folder-create-input');
    if (input) input.value = 'Reporting';
    root().querySelector('form')?.dispatchEvent(new Event('submit'));
    await fixture.whenStable();

    expect(seen).toEqual(['Reporting']);
  });

  /** A folder belongs to a space; without one there is nothing to create it in. */
  it('says why a folder cannot be created instead of offering a form that fails', async () => {
    fixture.componentRef.setInput('canCreate', false);

    await open();

    expect(root().querySelector('[data-testid="folder-create-open"]')).toBeNull();
    expect(root().querySelector('[data-testid="folder-create-blocked"]')).not.toBeNull();
  });

  it('renames the folder being edited', async () => {
    await open();
    await click('[data-testid="folder-edit"]');
    const input = root().querySelector<HTMLInputElement>('[data-testid="folder-rename-input"]');
    if (input) input.value = 'Performance';
    root().querySelector('form')?.dispatchEvent(new Event('submit'));
    await fixture.whenStable();

    expect(actions.renamed).toEqual([{ id: 'perf', name: 'Performance' }]);
  });

  it('offers the five colours and applies the one picked', async () => {
    await open();
    await click('[data-testid="folder-edit"]');
    const swatches = [...root().querySelectorAll<HTMLElement>('[data-testid="folder-colour"]')];
    expect(swatches).toHaveLength(5);
    swatches[4].click();
    await fixture.whenStable();

    expect(actions.recoloured).toEqual([{ id: 'perf', colour: 'red' }]);
  });

  it('marks the colour the folder already carries', async () => {
    await open();
    await click('[data-testid="folder-edit"]');

    const on = root().querySelector<HTMLElement>('[data-testid="folder-colour"].on');
    expect(on?.className).toContain('is-amber');
  });

  /** Two steps: the WebView blocks on a native `confirm()`. */
  it('asks once before deleting, and says the notes stay', async () => {
    await open();
    await click('[data-testid="folder-edit"]');
    expect(root().querySelector('.editor-note')?.textContent).toContain('Les notes restent');

    await click('[data-testid="folder-delete"]');
    expect(actions.deleted).toEqual([]);

    await click('[data-testid="folder-delete"]');
    expect(actions.deleted).toEqual(['perf']);
  });

  /** ⚠️ Unlike a space, which cannot go without one: its cascade would take the notes. */
  it('offers no refuge to choose when deleting', async () => {
    await open();
    await click('[data-testid="folder-edit"]');

    expect(root().querySelector('select')).toBeNull();
  });
});
