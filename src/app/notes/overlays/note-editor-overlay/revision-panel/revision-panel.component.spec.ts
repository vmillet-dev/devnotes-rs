import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { NoteRevisionsStore } from '@core/state/note-revisions.store';
import { NotesStore } from '@core/state/notes.store';
import { createNote } from '@testing/note.fixture';
import { FakeNotesRepository } from '@testing/fake-notes-repository';
import { provideAppTesting } from '@testing/testing.providers';
import { RevisionPanelComponent } from './revision-panel.component';

describe('RevisionPanelComponent', () => {
  let fixture: ComponentFixture<RevisionPanelComponent>;
  let store: NoteRevisionsStore;
  let repository: FakeNotesRepository;

  const toggle = (): HTMLButtonElement | null =>
    fixture.nativeElement.querySelector('[data-testid="revisions-toggle"]');

  const rows = (): HTMLElement[] => [
    ...fixture.nativeElement.querySelectorAll('[data-testid="revision-row"]'),
  ];

  async function render(): Promise<void> {
    fixture = TestBed.createComponent(RevisionPanelComponent);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  }

  beforeEach(async () => {
    TestBed.resetTestingModule();
    repository = new FakeNotesRepository([createNote({ id: 'note-1', content: 'select 1' })]);
    TestBed.configureTestingModule({
      imports: [RevisionPanelComponent],
      providers: [provideAppTesting({ notesRepository: repository })],
    });
    store = TestBed.inject(NoteRevisionsStore);
    await render();
  });

  /** ⚠️ A panel always there and always empty is a feature nobody uses. */
  it('shows nothing at all for a note nobody has edited', async () => {
    await store.openFor('note-1');
    await fixture.whenStable();

    expect(toggle()).toBeNull();
  });

  it('appears once an edit has left a body behind, folded shut', async () => {
    await repository.update('note-1', { content: 'select 2' });
    await store.openFor('note-1');
    await fixture.whenStable();

    expect(toggle()).not.toBeNull();
    expect(toggle()!.getAttribute('aria-expanded')).toBe('false');
    expect(rows()).toHaveLength(0);
  });

  it('lists what was kept, once opened', async () => {
    await repository.update('note-1', { content: 'select 2' });
    await repository.update('note-1', { content: 'select 3' });
    await store.openFor('note-1');
    await fixture.whenStable();

    toggle()!.click();
    await fixture.whenStable();

    expect(rows()).toHaveLength(2);
    expect(rows()[0].textContent).toContain('caractères');
  });

  /**
   * ⚠️ No confirmation, deliberately: the body a restore replaces is kept first, so it is
   * as undoable as the edit that made it necessary. A guard in front of a reversible
   * gesture is how a safety net becomes a nuisance.
   */
  it('puts a body back on one click', async () => {
    await repository.update('note-1', { content: 'select 2' });
    await store.openFor('note-1');
    await fixture.whenStable();
    toggle()!.click();
    await fixture.whenStable();

    (fixture.nativeElement.querySelector('[data-testid="revision-restore"]') as HTMLButtonElement).click();
    await fixture.whenStable();

    // Read back through the history rather than through the note, which the fake keeps
    // to itself: the newest kept body is now the one the restore replaced, which is only
    // true if the older one went back onto the note.
    expect(store.revisions()).toHaveLength(2);
    const replaced = await repository.restoreRevision('note-1', store.revisions()[0].id);
    expect(replaced.content).toBe('select 2');
  });

  /**
   * ⚠️ The defect the end-to-end run found, and the reason this is worth a fast test too:
   * the editor's body draft is a `linkedSignal` on the note **id**, which does not change
   * when a body is put back underneath it. The field went on showing the version that had
   * just been replaced — and the commit on close wrote it straight back, so the restore
   * undid itself.
   */
  it('puts the restored row where the editor re-seeds its draft from', async () => {
    const notes = TestBed.inject(NotesStore);
    await repository.update('note-1', { content: 'select 2' });
    notes.openNote(createNote({ id: 'note-1', content: 'select 2' }));
    await store.openFor('note-1');
    await fixture.whenStable();
    toggle()!.click();
    await fixture.whenStable();
    const before = store.restored();

    (fixture.nativeElement.querySelector('[data-testid="revision-restore"]') as HTMLButtonElement).click();
    await fixture.whenStable();

    expect(notes.selectedNote()?.content).toBe('select 1');
    // The counter moves after the adoption, which is what re-seeds the field.
    expect(store.restored()).toBe(before + 1);
  });

  /** ⚠️ A history left over from the note before would offer to paste its body here. */
  it('drops the history when the editor moves to another note', async () => {
    await repository.update('note-1', { content: 'select 2' });
    await store.openFor('note-1');
    await fixture.whenStable();
    expect(toggle()).not.toBeNull();

    await store.openFor(null);
    await fixture.whenStable();

    expect(toggle()).toBeNull();
  });
});
