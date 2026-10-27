import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { describe, expect, it, vi } from 'vitest';
import { NotesStore } from '@core/state/notes.store';
import { createNotesHarness, visibleIds } from '@testing/notes-harness';
import { NoteEditorOverlayComponent } from './note-editor-overlay/note-editor-overlay.component';

/** The editor bound to the store as the page binds it: what Escape closes is what the store holds. */
@Component({
  selector: 'app-escape-host',
  imports: [NoteEditorOverlayComponent],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    @if (store.selectedNote(); as note) {
      <app-note-editor-overlay [note]="note" [session]="store.editorSession()" />
    }
  `,
})
class EscapeHostComponent {
  protected readonly store = inject(NotesStore);
}

describe('A new note closed with Escape', () => {
  it('is written with the title typed, the caret still in it', async () => {
    const { store, canvas, repository } = await createNotesHarness([]);
    const create = vi.spyOn(repository, 'create');
    const fixture = TestBed.createComponent(EscapeHostComponent);
    document.body.append(fixture.nativeElement);
    fixture.autoDetectChanges();

    store.createNote('snippet');
    await fixture.whenStable();
    const title = fixture.nativeElement.querySelector('[data-testid="editor-title"]') as HTMLInputElement;
    title.focus();
    title.value = 'Closed with Escape';
    title.dispatchEvent(new Event('input'));
    await fixture.whenStable();

    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    await fixture.whenStable();

    await vi.waitFor(() => expect(create).toHaveBeenCalledTimes(1));
    expect(create.mock.calls[0]?.[0]).toMatchObject({ title: 'Closed with Escape' });
    await vi.waitFor(() => expect(visibleIds(canvas)).toHaveLength(1));
    expect(fixture.nativeElement.querySelector('[data-testid="editor-title"]')).toBeNull();
    fixture.nativeElement.remove();
  });
});
