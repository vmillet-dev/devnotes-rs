import { TestBed } from '@angular/core/testing';
import { type MockInstance, afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { LibraryPreferencesService } from '@core/services/preferences/library-preferences.service';
import { NotesRepository } from '@core/data/notes.repository';
import { SpacesRepository } from '@core/data/spaces.repository';
import { NoteDraft } from '@core/model/note.model';
import { Space } from '@core/model/space.model';
import { FakeNotesRepository } from '@testing/fake-notes-repository';
import { FakeSpacesRepository } from '@testing/fake-spaces-repository';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { SampleNotesService } from './sample-notes.service';

describe('SampleNotesService', () => {
  let notes: FakeNotesRepository;
  let spaces: FakeSpacesRepository;
  let created: MockInstance<FakeNotesRepository['create']>;
  let service: SampleNotesService;
  let preferences: LibraryPreferencesService;

  /** The drafts handed to the repository, in the order they were written. */
  const drafts = (): NoteDraft[] => created.mock.calls.map(([draft]) => draft);

  function setUp(existingSpaces: Space[] = []): void {
    TestBed.resetTestingModule();
    notes = new FakeNotesRepository();
    spaces = new FakeSpacesRepository(existingSpaces);
    created = vi.spyOn(notes, 'create');
    TestBed.configureTestingModule({
      providers: [
        { provide: NotesRepository, useValue: notes },
        { provide: SpacesRepository, useValue: spaces },
        provideTranslocoTesting(),
      ],
    });
    service = TestBed.inject(SampleNotesService);
    preferences = TestBed.inject(LibraryPreferencesService);
  }

  beforeEach(() => {
    // ⚠️ `Date` alone, or the zoneless scheduler loses the `requestAnimationFrame` it needs.
    vi.useFakeTimers({ toFake: ['Date'] });
    vi.setSystemTime(new Date('2026-09-11T09:00:00Z'));
    setUp();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('files a space and its samples into a database that has never been written to', async () => {
    expect(await service.seedIfFirstRun()).not.toBeNull();

    expect(notes.seededSamples?.spaceName).toBe('Découverte');
    expect(drafts()).toHaveLength(4);
    // ⚠️ The drafts carry no space: it does not exist when they are composed, and the
    // command that creates it is the one that files them into it.
    expect(notes.seededSamples?.notes.every(({ draft }) => draft.spaceId === '')).toBe(true);
  });

  /**
   * A fresh install opens on a board with something on it: the switch would otherwise show
   * an empty canvas, and the feature would be finished and undiscoverable.
   */
  it('arrives already arranged, with one note left loose on purpose', async () => {
    await service.seedIfFirstRun();

    expect(notes.seededSamples?.folders).toEqual(['Snippets', 'Prise en main']);
    // ⚠️ Indexes, not ids: the folders do not exist when these are composed.
    expect(notes.seededSamples?.notes.map((note) => note.folder)).toEqual([1, 0, undefined, 0]);
  });

  /** ⚠️ "No folder" is a legitimate state, and the first launch shows it rather than
   *  describing it. */
  it('leaves the checklist unfiled, so the loose area is not empty either', async () => {
    await service.seedIfFirstRun();

    const loose = notes.seededSamples?.notes.filter((note) => note.folder === undefined) ?? [];
    expect(loose).toHaveLength(1);
    expect(loose[0]?.draft.kind).toBe('checklist');
  });

  it('carries one feature per sample', async () => {
    await service.seedIfFirstRun();
    const [welcome, snippet, checklist, code] = drafts();

    expect(welcome.pinned).toBe(true);
    expect(snippet.content).toContain('{{host}}');
    expect(snippet.content).toContain('{{port=5432}}');
    expect(checklist.kind).toBe('checklist');
    expect(checklist.items).toHaveLength(6);
    expect(checklist.items.every((item) => !item.done)).toBe(true);
    expect(code.lifecycle.kind).toBe('expires');
  });

  it('dates the deadline to the end of a local day', async () => {
    // Midnight would make a note dated today expired on the spot.
    await service.seedIfFirstRun();
    const lifecycle = drafts()[3].lifecycle;

    expect(lifecycle.kind).toBe('expires');
    if (lifecycle.kind !== 'expires') return;
    expect(lifecycle.at.getHours()).toBe(23);
    expect(lifecycle.at.getMinutes()).toBe(59);
    expect(lifecycle.at.getDate()).toBe(new Date('2026-09-18T09:00:00Z').getDate());
  });

  it('spells out the languages rather than leaving detection to guess', async () => {
    await service.seedIfFirstRun();

    expect(drafts().map((draft) => draft.language)).toEqual(['md', 'sh', 'txt', 'ts']);
  });

  it('never offers the samples twice', async () => {
    await service.seedIfFirstRun();

    expect(await service.seedIfFirstRun()).toBeNull();
    expect(drafts()).toHaveLength(4);
  });

  it('leaves an existing installation alone, and stops looking', async () => {
    setUp([{ id: 'space-1', name: 'Perso', pinned: false }]);

    expect(await service.seedIfFirstRun()).toBeNull();
    expect(drafts()).toHaveLength(0);
    // Marked, so the check does not run on every launch from now on.
    expect(preferences.read('devnotes.notes.samplesSeeded')).not.toBeNull();
  });

  it('stays silent when there is no database to write to', async () => {
    // jsdom has no bridge; the canvas reports its own failure.
    spaces.failNext = new Error('no bridge');

    await expect(service.seedIfFirstRun()).resolves.toBeNull();
    expect(drafts()).toHaveLength(0);
  });

  /**
   * ⚠️ The opposite of what this asserted before, and the point of the ticket: the space
   * and its notes are one write now, so a failure leaves neither — and "it is seeded" is
   * written only once it is true, which lets the next launch try again.
   */
  it('seeds again after a failure, instead of closing the door on an empty canvas', async () => {
    notes.failNext = new Error('disk full');

    expect(await service.seedIfFirstRun()).toBeNull();
    expect(preferences.read('devnotes.notes.samplesSeeded')).toBeNull();

    expect(await service.seedIfFirstRun()).not.toBeNull();
    expect(notes.seededSamples?.notes).toHaveLength(4);
  });
});
