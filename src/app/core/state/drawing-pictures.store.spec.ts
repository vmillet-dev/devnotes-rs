import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { FakeNotesRepository } from '@testing/fake-notes-repository';
import { provideAppTesting } from '@testing/testing.providers';
import { DrawingPicturesStore } from './drawing-pictures.store';
import { NotesRevision } from './notes-revision';

describe('DrawingPicturesStore', () => {
  let repository: FakeNotesRepository;
  let store: DrawingPicturesStore;

  beforeEach(() => {
    repository = new FakeNotesRepository([]);
    repository.pictures.set('sketch', {
      thumbnail: 'data:image/png;base64,T',
      picture: 'data:image/png;base64,P',
    });
    TestBed.configureTestingModule({ providers: [provideAppTesting({ notesRepository: repository })] });
    store = TestBed.inject(DrawingPicturesStore);
  });

  it('asks for a card’s picture once per save of the drawing', async () => {
    expect(await store.thumbnail('sketch', 'at-1')).toBe('data:image/png;base64,T');
    expect(await store.thumbnail('sketch', 'at-1')).toBe('data:image/png;base64,T');
    await store.thumbnail('sketch', 'at-2');

    expect(repository.picturesAsked).toEqual(['sketch', 'sketch']);
  });

  it('asks for an embed’s picture again once any note is written', async () => {
    expect(await store.picture('sketch')).toBe('data:image/png;base64,P');
    await store.picture('sketch');
    TestBed.inject(NotesRevision).bump();
    await store.picture('sketch');

    expect(repository.picturesAsked).toEqual(['sketch', 'sketch']);
  });

  it('lists the drawings a Note can embed, and none when they cannot be had', async () => {
    repository.drawings = [{ id: 'sketch', title: 'Archi' }];

    expect(await store.list()).toEqual([{ id: 'sketch', title: 'Archi' }]);
    repository.failNext = new Error('locked');
    expect(await store.list()).toEqual([]);
  });

  it('answers nothing for a picture that cannot be had', async () => {
    repository.failNext = new Error('locked');

    expect(await store.thumbnail('sketch', 'at-1')).toBeNull();
    expect(await store.picture('gone')).toBeNull();
  });
});
