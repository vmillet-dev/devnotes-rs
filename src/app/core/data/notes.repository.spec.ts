import { TestBed } from '@angular/core/testing';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { commands } from '@core/ipc/bindings';
import { NotePriority } from '@core/model/note.model';
import { NotesRepository } from './notes.repository';

describe('NotesRepository', () => {
  afterEach(() => vi.restoreAllMocks());

  /** The batch answers what moved, and the undo hands exactly that back. */
  it('sets a priority, and puts back what the batch answered', async () => {
    const previous: NotePriority[] = [{ noteId: 'a', priority: 'low' }];
    const set = vi.spyOn(commands, 'setPriority').mockResolvedValue({ status: 'ok', data: previous });
    const restore = vi.spyOn(commands, 'restorePriorities').mockResolvedValue({ status: 'ok', data: 1 });
    const repository = TestBed.inject(NotesRepository);

    expect(await repository.setPriority(['a'], 'urgent')).toEqual(previous);
    expect(await repository.restorePriorities(previous)).toBe(1);
    expect(set).toHaveBeenCalledWith(['a'], 'urgent');
    expect(restore).toHaveBeenCalledWith(previous);
  });
});
