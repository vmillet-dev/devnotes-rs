import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { Folder } from '@core/model/folder.model';
import { FoldersStore } from '@core/state/folders.store';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { RecordingFolderActions } from '@testing/recording-actions';
import { FolderEditorComponent } from './folder-editor.component';

const PERF: Folder = {
  id: 'perf',
  spaceId: 'sql',
  name: 'Perf',
  colour: 'amber',
  createdAt: new Date('2026-01-01T10:00:00Z'),
  parentId: null,
};

describe('FolderEditorComponent', () => {
  let fixture: ComponentFixture<FolderEditorComponent>;
  let actions: RecordingFolderActions;
  let finished: number;

  function root(): HTMLElement {
    return fixture.nativeElement;
  }

  async function click(selector: string): Promise<void> {
    root().querySelector<HTMLElement>(selector)?.click();
    await fixture.whenStable();
  }

  async function rename(name: string): Promise<void> {
    const input = root().querySelector<HTMLInputElement>('[data-testid="folder-rename-input"]');
    if (input) input.value = name;
    root().querySelector('form')?.dispatchEvent(new Event('submit'));
    await fixture.whenStable();
  }

  beforeEach(async () => {
    actions = new RecordingFolderActions();
    finished = 0;
    TestBed.configureTestingModule({
      imports: [FolderEditorComponent],
      providers: [provideTranslocoTesting(), { provide: FoldersStore, useValue: actions }],
    });
    fixture = TestBed.createComponent(FolderEditorComponent);
    fixture.componentRef.setInput('folder', PERF);
    fixture.componentInstance.finished.subscribe(() => (finished += 1));
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  it('renames the folder itself, then says it is done', async () => {
    await rename('Performance');

    expect(actions.renamed).toEqual([{ id: 'perf', name: 'Performance' }]);
    expect(finished).toBe(1);
  });

  it('renames nothing for a blank name, and stays open', async () => {
    await rename('   ');

    expect(actions.renamed).toEqual([]);
    expect(finished).toBe(0);
  });

  it('recolours without closing, so the new swatch can be seen', async () => {
    await click('[data-colour="red"]');

    expect(actions.recoloured).toEqual([{ id: 'perf', colour: 'red' }]);
    expect(finished).toBe(0);
  });

  it('does not recolour to the colour it already carries', async () => {
    await click('[data-colour="amber"]');

    expect(actions.recoloured).toEqual([]);
  });

  it('deletes on the second click only', async () => {
    await click('[data-testid="folder-delete"]');
    expect(actions.deleted).toEqual([]);

    await click('[data-testid="folder-delete"]');
    expect(actions.deleted).toEqual(['perf']);
    expect(finished).toBe(1);
  });

  describe('a sub-folder', () => {
    let made: Folder[];

    async function nest(name: string): Promise<void> {
      await click('[data-testid="folder-subfolder-open"]');
      const input = root().querySelector<HTMLInputElement>('[data-testid="folder-subfolder-input"]');
      if (input) input.value = name;
      input?.form?.dispatchEvent(new Event('submit'));
      await fixture.whenStable();
    }

    beforeEach(() => {
      made = [];
      fixture.componentInstance.created.subscribe((folder) => made.push(folder));
    });

    it('opens its field on demand, focused', async () => {
      expect(root().querySelector('[data-testid="folder-subfolder-input"]')).toBeNull();

      await click('[data-testid="folder-subfolder-open"]');

      const input = root().querySelector('[data-testid="folder-subfolder-input"]');
      expect(document.activeElement).toBe(input);
    });

    it('is made inside this folder, then the panel says it is done', async () => {
      await nest('Index');

      expect(actions.created).toEqual([{ name: 'Index', parentId: 'perf' }]);
      expect(made.map((folder) => folder.name)).toEqual(['Index']);
      expect(finished).toBe(1);
    });

    it('is not asked for under a blank name', async () => {
      await nest('  ');

      expect(actions.created).toEqual([]);
      expect(finished).toBe(0);
    });

    it('leaves the field open when the name is refused', async () => {
      actions.accepts = false;

      await nest('Index');

      expect(made).toEqual([]);
      expect(finished).toBe(0);
      expect(root().querySelector('[data-testid="folder-subfolder-input"]')).not.toBeNull();
    });

    it('says nothing once the panel has gone, the folder made all the same', async () => {
      const warn = vi.spyOn(console, 'warn').mockImplementation(() => undefined);
      await click('[data-testid="folder-subfolder-open"]');
      const input = root().querySelector<HTMLInputElement>('[data-testid="folder-subfolder-input"]');
      if (input) input.value = 'Index';
      input?.form?.dispatchEvent(new Event('submit'));
      fixture.destroy();
      await new Promise((resolve) => setTimeout(resolve));

      expect(actions.created).toEqual([{ name: 'Index', parentId: 'perf' }]);
      expect(warn).not.toHaveBeenCalled();
      warn.mockRestore();
    });
  });

  it('offers to select the notes only where they are on screen', async () => {
    expect(root().querySelector('[data-testid="folder-select-notes"]')).toBeNull();

    fixture.componentRef.setInput('selectableCount', 3);
    await fixture.whenStable();

    expect(root().querySelector('[data-testid="folder-select-notes"]')).not.toBeNull();
  });
});
