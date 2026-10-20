import { TestBed } from '@angular/core/testing';
import { describe, expect, it, vi } from 'vitest';
import { Folder } from '@core/model/folder.model';
import { FakeBoardRepository } from '@testing/fake-board-repository';
import { FakeFoldersRepository } from '@testing/fake-folders-repository';
import { provideAppTesting } from '@testing/testing.providers';
import { UndoStore } from './undo.store';

const PERF: Folder = {
  id: 'perf',
  spaceId: 'sql',
  name: 'Perf',
  colour: 'blue',
  createdAt: new Date('2026-01-01'),
  parentId: 'base',
};

function create(): { undo: UndoStore; folders: FakeFoldersRepository; board: FakeBoardRepository } {
  const folders = new FakeFoldersRepository([{ ...PERF, id: 'base', name: 'Base', parentId: null }, PERF]);
  const board = new FakeBoardRepository();
  TestBed.configureTestingModule({
    providers: [provideAppTesting({ foldersRepository: folders, boardRepository: board })],
  });
  return { undo: TestBed.inject(UndoStore), folders, board };
}

describe('UndoStore', () => {
  it('puts a moved folder back under the parent it left', async () => {
    const { undo, folders } = create();
    const was = await folders.move('perf', null);

    undo.record({ kind: 'folderMove', was, name: 'Perf', into: null, count: 1 });
    await undo.revert();

    await vi.waitFor(async () =>
      expect((await folders.loadAll('sql')).find((each) => each.id === 'perf')?.parentId).toBe('base'),
    );
  });

  it('puts a tidied board back on the board it was made on', async () => {
    const { undo, board } = create();
    const layout = { zones: [], cards: [{ noteId: 'n', position: { x: 16, y: 16 } }] };

    undo.record({ kind: 'arrange', layout, rootId: 'base', count: 1 });
    await undo.revert();

    expect(board.saved).toEqual([{ rootId: 'base', ...layout }]);
  });
});
