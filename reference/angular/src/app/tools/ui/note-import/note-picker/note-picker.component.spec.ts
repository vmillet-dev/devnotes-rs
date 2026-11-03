import { ComponentFixture, TestBed } from '@angular/core/testing';
import { describe, expect, it, vi } from 'vitest';
import { NotesQuery } from '@core/model/note.model';
import { ANY_NOTE, NoteInput, STRUCTURED_SNIPPET } from '@core/services/tools/tool.model';
import { FakeNotesRepository } from '@testing/fake-notes-repository';
import { createNote } from '@testing/note.fixture';
import { provideAppTesting } from '@testing/testing.providers';
import { NotePickerComponent, PickedNote } from './note-picker.component';

describe('NotePickerComponent', () => {
  async function open(accepts: NoteInput = STRUCTURED_SNIPPET): Promise<{
    fixture: ComponentFixture<NotePickerComponent>;
    picked: PickedNote[];
  }> {
    TestBed.resetTestingModule();
    const notes = new FakeNotesRepository([
      createNote({
        id: 'staging',
        title: 'config.staging.json',
        language: 'json',
        content: '{"replicas":2}',
      }),
      createNote({
        id: 'release',
        title: 'Release',
        kind: 'checklist',
        content: '',
        items: [{ text: 'Tag', done: true }],
      }),
    ]);
    TestBed.configureTestingModule({
      imports: [NotePickerComponent],
      providers: [provideAppTesting({ notesRepository: notes })],
    });
    const fixture = TestBed.createComponent(NotePickerComponent);
    fixture.componentRef.setInput('accepts', accepts);
    const picked: PickedNote[] = [];
    fixture.componentInstance.picked.subscribe((note) => picked.push(note));
    fixture.autoDetectChanges();
    await fixture.whenStable();
    return { fixture, picked };
  }

  function option(fixture: ComponentFixture<NotePickerComponent>, id: string): HTMLButtonElement | null {
    return fixture.nativeElement.querySelector(`[data-note-id="${id}"]`);
  }

  it('asks Rust for the kinds and languages the input accepts, in every space', async () => {
    const query = vi.spyOn(FakeNotesRepository.prototype, 'query');
    await open();

    await vi.waitFor(() => expect(query).toHaveBeenCalled());
    const asked = query.mock.calls[0]![0] as NotesQuery;
    query.mockRestore();
    expect([asked.spaceId, asked.languages, asked.kinds]).toEqual([
      null,
      ['json', 'yml', 'toml', 'xml'],
      ['snippet'],
    ]);
  });

  it('says what it lists before anything is typed', async () => {
    const { fixture } = await open();
    const accepted = (): string =>
      fixture.nativeElement.querySelector('[data-testid="note-picker-accepted"]').textContent.trim();

    expect(accepted()).toBe('Snippets JSON, YAML, TOML ou XML');

    fixture.componentRef.setInput('accepts', ANY_NOTE);
    await fixture.whenStable();
    expect(accepted()).toBe('Toutes les notes');
  });

  it('hands over the snippet chosen, read whole', async () => {
    const { fixture, picked } = await open();
    await vi.waitFor(() => expect(option(fixture, 'staging')).not.toBeNull());

    option(fixture, 'staging')!.click();

    await vi.waitFor(() =>
      expect(picked).toEqual([{ title: 'config.staging.json', content: '{"replicas":2}', language: 'json' }]),
    );
  });

  it('gives a todo list as the Markdown copying it gives', async () => {
    const { fixture, picked } = await open(ANY_NOTE);
    await vi.waitFor(() => expect(option(fixture, 'release')).not.toBeNull());

    option(fixture, 'release')!.click();

    await vi.waitFor(() =>
      expect(picked).toEqual([{ title: 'Release', content: '- [x] Tag', language: null }]),
    );
  });

  it('takes the first with Enter in the search, and walks the rows with the arrows', async () => {
    const { fixture, picked } = await open(ANY_NOTE);
    await vi.waitFor(() => expect(option(fixture, 'release')).not.toBeNull());
    const search: HTMLInputElement = fixture.nativeElement.querySelector(
      '[data-testid="note-picker-search"]',
    );

    search.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true }));
    const first = fixture.nativeElement.querySelector('[data-testid="note-picker-option"]');
    expect(document.activeElement).toBe(first);
    first.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowUp', bubbles: true }));
    expect(document.activeElement).toBe(search);

    search.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));

    await vi.waitFor(() => expect(picked).toHaveLength(1));
    expect(picked[0]!.title).toBe(first.querySelector('.note-title').textContent.trim());
  });
});
