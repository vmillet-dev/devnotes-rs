import { ComponentFixture, TestBed } from '@angular/core/testing';
import { describe, expect, it, vi } from 'vitest';
import { NotesQuery } from '@core/model/note.model';
import { FakeNotesRepository } from '@testing/fake-notes-repository';
import { createNote } from '@testing/note.fixture';
import { provideAppTesting } from '@testing/testing.providers';
import { JsonNotePickerComponent, PickedNote } from './json-note-picker.component';

describe('JsonNotePickerComponent', () => {
  async function open(): Promise<{
    fixture: ComponentFixture<JsonNotePickerComponent>;
    notes: FakeNotesRepository;
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
    ]);
    TestBed.configureTestingModule({
      imports: [JsonNotePickerComponent],
      providers: [provideAppTesting({ notesRepository: notes })],
    });
    const fixture = TestBed.createComponent(JsonNotePickerComponent);
    const picked: PickedNote[] = [];
    fixture.componentInstance.picked.subscribe((note) => picked.push(note));
    fixture.autoDetectChanges();
    await fixture.whenStable();
    return { fixture, notes, picked };
  }

  it('asks for the JSON snippets of every space', async () => {
    const query = vi.spyOn(FakeNotesRepository.prototype, 'query');
    await open();

    await vi.waitFor(() => expect(query).toHaveBeenCalled());
    const asked = query.mock.calls[0]![0] as NotesQuery;
    query.mockRestore();
    expect([asked.spaceId, asked.languages, asked.kinds]).toEqual([null, ['json'], ['snippet']]);
  });

  it('hands over the snippet chosen, read whole', async () => {
    const { fixture, picked } = await open();
    await vi.waitFor(() =>
      expect(fixture.nativeElement.querySelector('[data-note-id="staging"]')).not.toBeNull(),
    );

    fixture.nativeElement.querySelector('[data-note-id="staging"]').click();

    await vi.waitFor(() =>
      expect(picked).toEqual([{ title: 'config.staging.json', content: '{"replicas":2}' }]),
    );
  });
});
