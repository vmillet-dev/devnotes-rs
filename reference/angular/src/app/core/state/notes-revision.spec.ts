import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { NotesRevision } from './notes-revision';

describe('NotesRevision', () => {
  let revision: NotesRevision;

  beforeEach(() => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({});
    revision = TestBed.inject(NotesRevision);
  });

  it('starts somewhere rather than at nothing', () => {
    expect(revision.current()).toBe(0);
  });

  /**
   * Every bump has to be a new value, never a toggle: `NotesQueryStore` reads this
   * among its query parameters, and a value that came back to one already seen would
   * compare equal and leave the canvas showing what it had.
   */
  it('moves to a value it has never had before, on every bump', () => {
    const seen = new Set([revision.current()]);

    for (let bump = 0; bump < 5; bump++) {
      revision.bump();
      expect(seen.has(revision.current())).toBe(false);
      seen.add(revision.current());
    }
  });

  it('is one counter for the whole application', () => {
    revision.bump();

    // The trash, the tag manager and the library all bump the same instance.
    expect(TestBed.inject(NotesRevision).current()).toBe(revision.current());
  });

  it('hands out a signal nobody else can write', () => {
    const current = revision.current;

    expect('set' in current).toBe(false);
    expect('update' in current).toBe(false);
  });
});
