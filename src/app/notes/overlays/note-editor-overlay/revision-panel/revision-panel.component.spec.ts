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

  function click(hook: string, index = 0): void {
    (fixture.nativeElement.querySelectorAll(`[data-testid="${hook}"]`)[index] as HTMLButtonElement).click();
  }

  function text(hook: string): string {
    return fixture.nativeElement.querySelector(`[data-testid="${hook}"]`)?.textContent ?? '';
  }

  async function openHistory(...bodies: string[]): Promise<void> {
    for (const body of bodies) {
      await repository.update('note-1', { content: body });
    }
    await store.openFor('note-1');
    await fixture.whenStable();
    toggle()!.click();
    await fixture.whenStable();
  }

  const current = (): string | undefined => repository.contentOf('note-1');

  /**
   * ⚠️ Going back is irreversible — the current text and every newer version go — so a
   * row opens a preview and never restores on its own (#325).
   */
  it('opens a preview on a click, and changes nothing yet', async () => {
    await openHistory('select 2');

    click('revision-open');
    await fixture.whenStable();

    expect(fixture.nativeElement.querySelector('[data-testid="revision-preview"]')).not.toBeNull();
    expect(rows()).toHaveLength(0);
    expect(text('revision-diff')).toContain('select 1');
    expect(current()).toBe('select 2');
  });

  it('says what going back would erase before it does', async () => {
    await openHistory('select 2', 'select 3');

    click('revision-open', 1);
    await fixture.whenStable();

    expect(text('revision-consequence')).toContain('la version plus récente');
  });

  /** The reporter's case: A → B → C, back to B, and C no longer exists. */
  it('goes back from the preview, and the history loses that version and every newer one', async () => {
    await openHistory('select 2', 'select 3');

    click('revision-open', 0);
    await fixture.whenStable();
    click('revision-restore');
    await fixture.whenStable();

    expect(current()).toBe('select 2');
    expect(store.revisions()).toHaveLength(1);
    expect(fixture.nativeElement.querySelector('[data-testid="revision-preview"]')).toBeNull();
  });

  it('goes back to the list without touching anything', async () => {
    await openHistory('select 2');
    click('revision-open');
    await fixture.whenStable();

    click('revision-back');
    await fixture.whenStable();

    expect(rows()).toHaveLength(1);
    expect(current()).toBe('select 2');
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
    await notes.openNote('note-1');
    await openHistory();
    const before = store.restored();

    click('revision-open');
    await fixture.whenStable();
    click('revision-restore');
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
