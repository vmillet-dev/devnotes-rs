import { ComponentFixture, TestBed } from '@angular/core/testing';
import { describe, expect, it, vi } from 'vitest';
import { Folder } from '@core/model/folder.model';
import { Space } from '@core/model/space.model';
import { StatusNotifier } from '@core/services/notifications/status.service';
import { ToolResult } from '@core/services/tools/tool.model';
import { FoldersStore } from '@core/state/folders.store';
import { NotesRevision } from '@core/state/notes-revision';
import { SpacesStore } from '@core/state/spaces.store';
import { FakeNotesRepository } from '@testing/fake-notes-repository';
import { provideAppTesting } from '@testing/testing.providers';
import { SaveAsNoteDialogComponent } from './save-as-note-dialog.component';

const SPACES: readonly Space[] = [
  { id: 'perso', name: 'Personnel', pinned: false },
  { id: 'work', name: 'Travail', pinned: false },
];

const FOLDERS: readonly Folder[] = [
  { id: 'scripts', spaceId: 'work', name: 'Scripts', colour: 'blue', createdAt: new Date('2026-01-01') },
];

const RESULT: ToolResult = {
  title: { key: 'tools.slug.noteTitle', params: { text: 'Été 2026' } },
  kind: 'snippet',
  language: 'json',
  content: '{ "slug": "ete-2026" }',
};

describe('SaveAsNoteDialogComponent', () => {
  let fixture: ComponentFixture<SaveAsNoteDialogComponent>;
  let notes: FakeNotesRepository;
  let closed: number;

  async function open(activeSpace: string | null = null, activeFolder: string | null = null): Promise<void> {
    TestBed.resetTestingModule();
    notes = new FakeNotesRepository([]);
    TestBed.configureTestingModule({
      imports: [SaveAsNoteDialogComponent],
      providers: [provideAppTesting({ notesRepository: notes, spaces: SPACES, folders: FOLDERS })],
    });
    const spaces = TestBed.inject(SpacesStore);
    const folders = TestBed.inject(FoldersStore);
    await vi.waitFor(() => expect(spaces.spaces()).toHaveLength(2));
    await vi.waitFor(() => expect(folders.allFolders()).toHaveLength(1));
    spaces.selectSpace(activeSpace);
    folders.selectFolder(activeFolder);

    fixture = TestBed.createComponent(SaveAsNoteDialogComponent);
    fixture.componentRef.setInput('result', RESULT);
    fixture.componentRef.setInput('toolId', 'slug');
    closed = 0;
    fixture.componentInstance.closed.subscribe(() => closed++);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  }

  const field = (testid: string): HTMLInputElement =>
    fixture.nativeElement.querySelector(`[data-testid="${testid}"]`);
  const menuText = (kind: string): string =>
    fixture.nativeElement.querySelector(`[data-testid="choice-${kind}"]`).textContent.trim();

  function type(testid: string, text: string): void {
    field(testid).value = text;
    field(testid).dispatchEvent(new Event('input'));
  }

  async function submit(): Promise<void> {
    field('save-as-note-submit').click();
    await vi.waitFor(() => expect(closed).toBe(1));
  }

  it('proposes the tool title, in the space and the folder open in the notes', async () => {
    await open('work', 'scripts');

    expect(field('save-as-note-title').value).toBe('Slug de « Été 2026 »');
    expect(menuText('save-as-note-space')).toContain('Travail');
    expect(menuText('save-as-note-folder')).toContain('Scripts');
    expect(fixture.nativeElement.querySelector('[data-testid="save-as-note-what"]').textContent).toContain(
      'JSON',
    );
  });

  it('falls back on the first space, unfiled, when the notes show all of them', async () => {
    await open();

    expect(menuText('save-as-note-space')).toContain('Personnel');
    expect(menuText('save-as-note-folder')).toContain('Aucun dossier');
  });

  it('writes the result as the tool shaped it, where the dialog put it', async () => {
    await open('work', 'scripts');
    const create = vi.spyOn(notes, 'create');
    const bump = vi.spyOn(TestBed.inject(NotesRevision), 'bump');
    type('save-as-note-title', '  Slug de la page été  ');
    type('save-as-note-tags', 'web, seo , ');

    await submit();

    expect(create).toHaveBeenCalledWith(
      expect.objectContaining({
        title: 'Slug de la page été',
        spaceId: 'work',
        folderId: 'scripts',
        tags: ['web', 'seo'],
        kind: 'snippet',
        language: 'json',
        content: '{ "slug": "ete-2026" }',
        source: 'Outils / Générateur de slug',
      }),
    );
    expect(bump).toHaveBeenCalled();
    expect(TestBed.inject(StatusNotifier).status()).toEqual({
      key: 'tools.saved',
      params: { title: 'Slug de la page été', place: 'Travail › Scripts' },
    });
  });

  it('leaves the folder behind when another space is chosen', async () => {
    await open('work', 'scripts');
    const create = vi.spyOn(notes, 'create');

    fixture.nativeElement.querySelector('[data-testid="choice-save-as-note-space"]').click();
    await fixture.whenStable();
    fixture.nativeElement
      .querySelector('[data-testid="choice-panel-save-as-note-space"] [data-option-id="perso"]')
      .click();
    await fixture.whenStable();
    await submit();

    expect(create).toHaveBeenCalledWith(expect.objectContaining({ spaceId: 'perso', folderId: null }));
  });

  it('refuses a blank title', async () => {
    await open();

    type('save-as-note-title', '   ');
    await fixture.whenStable();

    expect(field('save-as-note-submit').disabled).toBe(true);
  });

  it('stays open when the write fails', async () => {
    await open();
    notes.failNext = new Error('disk full');

    field('save-as-note-submit').click();
    await vi.waitFor(() => expect(field('save-as-note-submit').disabled).toBe(false));

    expect(closed).toBe(0);
  });
});
