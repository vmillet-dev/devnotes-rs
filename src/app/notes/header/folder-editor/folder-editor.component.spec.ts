import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
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

  it('offers to select the notes only where they are on screen', async () => {
    expect(root().querySelector('[data-testid="folder-select-notes"]')).toBeNull();

    fixture.componentRef.setInput('selectableCount', 3);
    await fixture.whenStable();

    expect(root().querySelector('[data-testid="folder-select-notes"]')).not.toBeNull();
  });
});
