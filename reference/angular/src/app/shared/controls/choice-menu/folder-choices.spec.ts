import { describe, expect, it } from 'vitest';
import { Folder } from '@core/model/folder.model';
import { folderChoices } from './folder-choices';

function folder(id: string, name: string, parentId: string | null): Folder {
  return { id, spaceId: 'sql', name, colour: 'blue', createdAt: new Date('2026-01-01'), parentId };
}

describe('folderChoices', () => {
  it('offers the tree in order, a nested folder hinted by its path', () => {
    const choices = folderChoices([folder('perf', 'Perf', 'base'), folder('base', 'Base', null)]);

    expect(choices).toEqual([
      { id: 'base', name: 'Base', colour: 'blue' },
      { id: 'perf', name: 'Perf', colour: 'blue', hint: 'Base › Perf' },
    ]);
  });
});
