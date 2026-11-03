import { TestBed } from '@angular/core/testing';
import { afterEach, describe, expect, it } from 'vitest';
import { NoteSelectionStore } from '@core/state/note-selection.store';
import { focusCard } from './focus-card';

describe('focusCard', () => {
  function cards(...ids: string[]): void {
    document.body.innerHTML = ids
      .map(
        (id) =>
          `<div data-testid="note-card" data-note-id="${id}"><button data-testid="note-card-open"></button></div>`,
      )
      .join('');
  }

  const button = (id: string) =>
    document.querySelector(`[data-note-id="${id}"] [data-testid="note-card-open"]`);

  afterEach(() => (document.body.innerHTML = ''));

  it('gives the keyboard to the card asked for, the state following', () => {
    cards('a', 'b');
    const selection = TestBed.inject(NoteSelectionStore);

    expect(focusCard(selection, 'b')).toBe(true);
    expect(document.activeElement).toBe(button('b'));
    expect(selection.focusedNoteId()).toBe('b');
  });

  it('falls back on the first card, and says when there is none', () => {
    const selection = TestBed.inject(NoteSelectionStore);
    expect(focusCard(selection, null)).toBe(false);

    cards('a', 'b');
    expect(focusCard(selection, 'gone')).toBe(true);
    expect(document.activeElement).toBe(button('a'));
  });
});
