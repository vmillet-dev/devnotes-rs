import { describe, expect, it } from 'vitest';
import { Folder, canMoveFolder, childrenOf, pathOf, subtreeOf, treeOrder } from './folder.model';

function folder(id: string, name: string, parentId: string | null, spaceId = 'sql'): Folder {
  return { id, spaceId, name, colour: 'blue', createdAt: new Date('2026-01-01'), parentId };
}

// sql: Base › (Perf › Index, Migrations), Angular; other: Ops
const TREE: readonly Folder[] = [
  folder('base', 'Base', null),
  folder('perf', 'Perf', 'base'),
  folder('index', 'Index', 'perf'),
  folder('migr', 'migrations', 'base'),
  folder('angular', 'Angular', null),
  folder('ops', 'Ops', null, 'other'),
];

describe('the folder tree', () => {
  it('lists siblings by name, whatever their case', () => {
    expect(childrenOf(TREE, 'base').map((each) => each.name)).toEqual(['migrations', 'Perf']);
    expect(childrenOf(TREE, null).map((each) => each.id)).toEqual(['angular', 'base', 'ops']);
  });

  it('reads a path from the top of the space down to the folder', () => {
    expect(pathOf(TREE, 'index').map((each) => each.id)).toEqual(['base', 'perf', 'index']);
    expect(pathOf(TREE, 'gone')).toEqual([]);
  });

  it('walks a cycle a damaged list makes once', () => {
    const looped = [folder('a', 'A', 'b'), folder('b', 'B', 'a')];

    expect(pathOf(looped, 'a').map((each) => each.id)).toEqual(['b', 'a']);
    expect([...subtreeOf(looped, 'a')].sort()).toEqual(['a', 'b']);
  });

  it('holds a folder and everything under it in its sub-tree', () => {
    expect([...subtreeOf(TREE, 'base')].sort()).toEqual(['base', 'index', 'migr', 'perf']);
  });

  it('lets a folder go anywhere in its space but into itself or where it already is', () => {
    expect(canMoveFolder(TREE, 'perf', 'angular')).toBe(true);
    expect(canMoveFolder(TREE, 'perf', null)).toBe(true);
    expect(canMoveFolder(TREE, 'base', 'index')).toBe(false);
    expect(canMoveFolder(TREE, 'base', 'base')).toBe(false);
    expect(canMoveFolder(TREE, 'perf', 'base')).toBe(false);
    expect(canMoveFolder(TREE, 'perf', 'ops')).toBe(false);
    expect(canMoveFolder(TREE, 'gone', null)).toBe(false);
  });

  it('orders the whole tree depth first, each folder with the path down to it', () => {
    const ordered = treeOrder(TREE.filter((each) => each.spaceId === 'sql'));

    expect(ordered.map(({ folder: each }) => each.id)).toEqual(['angular', 'base', 'migr', 'perf', 'index']);
    expect(ordered.at(-1)?.path.map((each) => each.name)).toEqual(['Base', 'Perf', 'Index']);
  });
});
