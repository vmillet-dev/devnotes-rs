import { describe, expect, it } from 'vitest';
import { HttpItem, HttpTree } from '@core/model/http.model';
import {
  collection as collectionNode,
  folder as folderNode,
  request as requestNode,
  sampleTree,
} from '@testing/http-tree.fixture';
import {
  dropMove,
  findRequest,
  keyMove,
  landing,
  railRows,
  refusal,
  requestsUnder,
  savePlaces,
} from './http-tree';

const folder = (id: string): HttpItem => ({ kind: 'folder', id });
const request = (id: string): HttpItem => ({ kind: 'request', id });
const collection = (id: string): HttpItem => ({ kind: 'collection', id });

describe('the HTTP rail rows', () => {
  it('lay the tree flat, each row at its depth, a request with its method or its kind', () => {
    const rows = railRows(sampleTree(), new Set());

    expect(
      rows.map((row) => `${'  '.repeat(row.depth)}${row.badge ?? ''}${row.badge ? ' ' : ''}${row.id}`),
    ).toEqual([
      'API',
      '  Auth',
      '    GET Login',
      '    GET Logout',
      '  Factures',
      '    Archives',
      '      GET Old',
      '  POST Health',
      'Catalogue',
      '  GQL Produits',
      'Flux',
      '  WS Live',
    ]);
    expect(rows.find((row) => row.id === 'Old')).toMatchObject({ collectionId: 'API', folderId: 'Archives' });
  });

  it('leave out what a collapsed collection or folder holds', () => {
    const rows = railRows(sampleTree(), new Set(['Factures', 'Catalogue']));

    expect(rows.map((row) => row.id)).toEqual([
      'API',
      'Auth',
      'Login',
      'Logout',
      'Factures',
      'Health',
      'Catalogue',
      'Flux',
      'Live',
    ]);
    expect(rows.find((row) => row.id === 'Factures')?.expanded).toBe(false);
  });
});

describe('a keyboard move', () => {
  it('goes up and down within its parent, and stops at either end', () => {
    expect(keyMove(sampleTree(), request('Logout'), 'up')).toEqual({
      kind: 'move',
      item: request('Logout'),
      place: { collectionId: 'API', folderId: 'Auth', index: 0 },
    });
    expect(keyMove(sampleTree(), folder('Auth'), 'down')).toEqual({
      kind: 'move',
      item: folder('Auth'),
      place: { collectionId: 'API', folderId: null, index: 1 },
    });
    expect(keyMove(sampleTree(), request('Login'), 'up')).toBeNull();
    expect(keyMove(sampleTree(), request('Health'), 'down')).toBeNull();
  });

  it('goes into the folder above it, at its end, and out of its folder after it', () => {
    expect(keyMove(sampleTree(), request('Health'), 'in')).toEqual({
      kind: 'move',
      item: request('Health'),
      place: { collectionId: 'API', folderId: 'Factures', index: 1 },
    });
    expect(keyMove(sampleTree(), request('Old'), 'out')).toEqual({
      kind: 'move',
      item: request('Old'),
      place: { collectionId: 'API', folderId: 'Factures', index: 1 },
    });
    expect(keyMove(sampleTree(), folder('Auth'), 'in')).toBeNull();
    expect(keyMove(sampleTree(), request('Health'), 'out')).toBeNull();
  });

  it('reorders a collection among the others, never into anything', () => {
    expect(keyMove(sampleTree(), collection('Catalogue'), 'up')).toEqual({
      kind: 'reorder',
      id: 'Catalogue',
      index: 0,
    });
    expect(keyMove(sampleTree(), collection('Flux'), 'down')).toBeNull();
    expect(keyMove(sampleTree(), collection('API'), 'in')).toBeNull();
    expect(keyMove(sampleTree(), request('Absent'), 'up')).toBeNull();
  });
});

/** Read from the tree as it is after the move: Rust renumbered it, and the front never guesses. */
describe('where a keyboard move landed', () => {
  it('names the folder a row went into, and the collection it came out to the root of', () => {
    const inside: HttpTree = {
      collections: [
        collectionNode('API', [
          folderNode('Factures', 'API', [
            requestNode('Old', 'API', 'Factures'),
            requestNode('Health', 'API', 'Factures'),
          ]),
        ]),
      ],
    };
    expect(landing(inside, request('Health'), 'in')).toEqual({
      parentId: 'Factures',
      where: 'into',
      other: 'Factures',
    });
    // Out of a folder that is itself in a folder: still inside something, and said so.
    expect(landing(inside, request('Old'), 'out')).toEqual({
      parentId: 'Factures',
      where: 'into',
      other: 'Factures',
    });

    const out: HttpTree = {
      collections: [collectionNode('API', [folderNode('Factures', 'API'), requestNode('Old', 'API')])],
    };
    expect(landing(out, request('Old'), 'out')).toEqual({ parentId: 'API', where: 'root', other: 'API' });
  });

  it('names the row that was passed, going up or down', () => {
    expect(landing(sampleTree(), request('Login'), 'up')).toEqual({
      parentId: 'Auth',
      where: 'before',
      other: 'Logout',
    });
    expect(landing(sampleTree(), request('Logout'), 'down')).toEqual({
      parentId: 'Auth',
      where: 'after',
      other: 'Login',
    });
    expect(landing(sampleTree(), folder('Factures'), 'down')).toEqual({
      parentId: 'API',
      where: 'after',
      other: 'Auth',
    });
  });

  it('reads a collection against its neighbour, held by nothing', () => {
    expect(landing(sampleTree(), collection('API'), 'up')).toEqual({
      parentId: null,
      where: 'before',
      other: 'Catalogue',
    });
    expect(landing(sampleTree(), collection('Flux'), 'down')).toEqual({
      parentId: null,
      where: 'after',
      other: 'Catalogue',
    });
    expect(landing(sampleTree(), collection('Absente'), 'up')).toBeNull();
    expect(landing({ collections: [collectionNode('Seule')] }, collection('Seule'), 'up')).toBeNull();
  });

  /** A key that does nothing looks unheard: the reason is what the rail says instead. */
  it('says why a move was not made', () => {
    expect(refusal(request('Login'), 'up')).toBe('first');
    expect(refusal(request('Health'), 'down')).toBe('last');
    expect(refusal(folder('Auth'), 'in')).toBe('noFolderAbove');
    expect(refusal(request('Health'), 'out')).toBe('atRoot');
    expect(refusal(collection('API'), 'up')).toBe('first');
    expect(refusal(collection('API'), 'in')).toBe('collection');
    expect(refusal(collection('API'), 'out')).toBe('collection');
  });
});

describe('a drop', () => {
  it('puts an item before or after a row, under that row’s parent', () => {
    expect(dropMove(sampleTree(), request('Health'), request('Login'), 'after')).toEqual({
      kind: 'move',
      item: request('Health'),
      place: { collectionId: 'API', folderId: 'Auth', index: 1 },
    });
    expect(dropMove(sampleTree(), request('Logout'), request('Login'), 'before')).toEqual({
      kind: 'move',
      item: request('Logout'),
      place: { collectionId: 'API', folderId: 'Auth', index: 0 },
    });
  });

  it('puts an item inside a folder or a collection, at its end, another collection included', () => {
    expect(dropMove(sampleTree(), request('Login'), folder('Factures'), 'inside')).toEqual({
      kind: 'move',
      item: request('Login'),
      place: { collectionId: 'API', folderId: 'Factures', index: 1 },
    });
    expect(dropMove(sampleTree(), folder('Auth'), collection('Catalogue'), 'before')).toEqual({
      kind: 'move',
      item: folder('Auth'),
      place: { collectionId: 'Catalogue', folderId: null, index: 1 },
    });
  });

  it('refuses a folder into itself or below itself, and a drop that changes nothing', () => {
    expect(dropMove(sampleTree(), folder('Factures'), folder('Archives'), 'inside')).toBeNull();
    expect(dropMove(sampleTree(), folder('Factures'), request('Old'), 'after')).toBeNull();
    expect(dropMove(sampleTree(), request('Login'), request('Logout'), 'before')).toBeNull();
    expect(dropMove(sampleTree(), request('Login'), request('Login'), 'after')).toBeNull();
  });

  it('reorders a collection among collections alone', () => {
    expect(dropMove(sampleTree(), collection('Flux'), collection('API'), 'before')).toEqual({
      kind: 'reorder',
      id: 'Flux',
      index: 0,
    });
    expect(dropMove(sampleTree(), collection('API'), collection('Catalogue'), 'before')).toBeNull();
    expect(dropMove(sampleTree(), collection('API'), folder('Auth'), 'inside')).toBeNull();
  });
});

describe('what a deletion takes', () => {
  const taken = (item: HttpItem) => [...requestsUnder(sampleTree(), item)].sort();

  it('is the request itself, all a folder holds at any depth, or all a collection holds', () => {
    expect(taken(request('Login'))).toEqual(['Login']);
    expect(taken(folder('Auth'))).toEqual(['Login', 'Logout']);
    // Old sits in Archives, a folder of Factures.
    expect(taken(folder('Factures'))).toEqual(['Old']);
    expect(taken(folder('Archives'))).toEqual(['Old']);
    expect(taken(collection('API'))).toEqual(['Health', 'Login', 'Logout', 'Old']);
    expect(taken(collection('Flux'))).toEqual(['Live']);
  });

  it('is nothing for what the tree does not hold', () => {
    expect(taken(request('Absente'))).toEqual([]);
    expect(taken(folder('Absent'))).toEqual([]);
    expect(taken(collection('Absente'))).toEqual([]);
  });
});

describe('findRequest', () => {
  it('answers a request with its name and where it sits, at any depth', () => {
    expect(findRequest(sampleTree(), 'Old')).toMatchObject({
      name: 'Old',
      collectionId: 'API',
      folderId: 'Archives',
    });
    expect(findRequest(sampleTree(), 'Live')).toMatchObject({ collectionId: 'Flux', folderId: null });
  });

  it('answers nothing for a folder, or for a request that is gone', () => {
    expect(findRequest(sampleTree(), 'Auth')).toBeNull();
    expect(findRequest(sampleTree(), 'Gone')).toBeNull();
  });
});

describe('savePlaces', () => {
  it('offers each collection, then each of its folders by its path', () => {
    expect(savePlaces(sampleTree()).map((place) => place.path.join(' / '))).toEqual([
      'API',
      'API / Auth',
      'API / Factures',
      'API / Factures / Archives',
      'Catalogue',
      'Flux',
    ]);
    expect(savePlaces(sampleTree())[3]).toMatchObject({ collectionId: 'API', folderId: 'Archives' });
  });
});
