import { describe, expect, it } from 'vitest';
import { HttpItem } from '@core/model/http.model';
import { sampleTree } from '@testing/http-tree.fixture';
import { dropMove, holdsRequest, keyMove, railRows } from './http-tree';

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
      '  QUERY Produits',
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

describe('holdsRequest', () => {
  it('says whether a request is still in the tree', () => {
    expect(holdsRequest(sampleTree(), 'Old')).toBe(true);
    expect(holdsRequest(sampleTree(), 'Auth')).toBe(false);
    expect(holdsRequest(sampleTree(), 'Gone')).toBe(false);
  });
});
