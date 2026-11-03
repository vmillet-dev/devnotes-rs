import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { LibraryPreferencesService } from '@core/services/preferences/library-preferences.service';
import { FakeHttpRepository } from '@testing/fake-http-repository';
import { sampleTree } from '@testing/http-tree.fixture';
import { provideAppTesting } from '@testing/testing.providers';
import { HttpCollectionsStore } from './http-collections.store';

describe('HttpCollectionsStore', () => {
  let http: FakeHttpRepository;

  function store(): HttpCollectionsStore {
    return TestBed.inject(HttpCollectionsStore);
  }

  beforeEach(() => {
    TestBed.resetTestingModule();
    http = new FakeHttpRepository();
    http.tree$ = sampleTree();
    TestBed.configureTestingModule({ providers: [provideAppTesting({ httpRepository: http })] });
  });

  it('reads the tree Rust built, and lays it out as rows', async () => {
    await store().load();

    expect(
      store()
        .rows()
        .map((row) => row.id),
    ).toContain('Old');
    expect(store().tree().collections).toHaveLength(3);
  });

  it('remembers a fold in the library’s preferences, and reads it back', async () => {
    await store().load();
    store().toggle('Factures');

    expect(
      store()
        .rows()
        .map((row) => row.id),
    ).not.toContain('Archives');
    expect(
      JSON.parse(TestBed.inject(LibraryPreferencesService).read('devnotes.notes.http.collapsed')!),
    ).toEqual(['Factures']);

    store().toggle('Factures');
    expect(store().collapsed().size).toBe(0);
  });

  it('starts from the folds the library kept, and survives a file written by hand', () => {
    TestBed.inject(LibraryPreferencesService).write('devnotes.notes.http.collapsed', '["API", 3]');
    expect([...store().collapsed()]).toEqual(['API']);

    TestBed.resetTestingModule();
    TestBed.configureTestingModule({ providers: [provideAppTesting({ httpRepository: http })] });
    TestBed.inject(LibraryPreferencesService).write('devnotes.notes.http.collapsed', '{not json');
    expect(store().collapsed().size).toBe(0);
  });

  it('creates under a parent it unfolds, reads the tree again, and answers a new request’s id', async () => {
    await store().load();
    store().toggle('Auth');

    await store().createFolder('API', null, 'Webhooks');
    const id = await store().createRequest('API', 'Auth', 'Refresh');

    expect(http.callsOf('createFolder')).toEqual([['API', null, 'Webhooks']]);
    expect(http.callsOf('createRequest')[0]?.[0]).toMatchObject({
      collectionId: 'API',
      folderId: 'Auth',
      name: 'Refresh',
      kind: 'http',
      method: 'GET',
    });
    expect(store().collapsed().has('Auth')).toBe(false);
    expect(id).toMatch(/^request-/);
    expect(http.callsOf('tree').length).toBeGreaterThanOrEqual(3);

    // Of the kind asked: a request is created as what it is, and stays so.
    await store().createRequest('API', null, 'Produits', 'graphql');
    await store().createRequest('API', null, 'Flux', 'websocket');
    expect(http.callsOf('createRequest').map((call) => (call[0] as { kind: string }).kind)).toEqual([
      'http',
      'graphql',
      'websocket',
    ]);

    http.failNext = new Error('disk');
    expect(await store().createRequest('API', null, 'Perdue')).toBeNull();
  });

  /** A request saved into a folded folder was lit where nobody could see it. */
  it('unfolds what hides a request, down to its folder, and nothing else', async () => {
    await store().load();
    store().toggle('API');
    store().toggle('Archives');
    store().toggle('Catalogue');

    store().reveal('Old');

    expect([...store().collapsed()]).toEqual(['Catalogue']);
    expect(
      store()
        .rows()
        .map((row) => row.id),
    ).toContain('Old');
    // Gone from the tree: nothing to unfold, and nothing breaks.
    store().reveal('Absente');
    expect([...store().collapsed()]).toEqual(['Catalogue']);
  });

  it('deletes through Rust and reads the tree again', async () => {
    await store().load();
    http.tree$ = { collections: [] };
    await store().delete({ kind: 'collection', id: 'API' });

    expect(http.callsOf('delete')).toEqual([[{ kind: 'collection', id: 'API' }]]);
    expect(store().rows()).toEqual([]);
  });

  it('sends a move or a reorder to Rust, unfolding the folder it lands in', async () => {
    await store().load();
    store().toggle('Factures');

    await store().apply({
      kind: 'move',
      item: { kind: 'request', id: 'Health' },
      place: { collectionId: 'API', folderId: 'Factures', index: 1 },
    });
    await store().apply({ kind: 'reorder', id: 'Flux', index: 0 });

    expect(http.callsOf('move')).toEqual([
      [
        { kind: 'request', id: 'Health' },
        { collectionId: 'API', folderId: 'Factures', index: 1 },
      ],
    ]);
    expect(http.callsOf('reorderCollection')).toEqual([['Flux', 0]]);
    expect(store().collapsed().has('Factures')).toBe(false);
  });

  /** An empty tree means nothing before a first read: what follows the tree waits for one. */
  it('says whether the tree was read at least once', async () => {
    expect(store().loaded()).toBe(false);

    http.failNext = new Error('disk');
    await store().load();
    expect(store().loaded()).toBe(false);

    await store().load();
    expect(store().loaded()).toBe(true);
  });

  /** « héritée du dossier Auth »: the name is in what its requests inherit from. */
  it('has what is inherited read again once a folder or a collection is renamed, not a request', async () => {
    const before = store().settingsRevision();

    await store().rename({ kind: 'request', id: 'Login' }, 'Connexion');
    expect(store().settingsRevision()).toBe(before);

    await store().rename({ kind: 'folder', id: 'Auth' }, 'Authentification');
    await store().rename({ kind: 'collection', id: 'API' }, 'API v2');
    expect(store().settingsRevision()).toBe(before + 2);
  });

  it('renames, copies and counts through Rust, and says when a write failed', async () => {
    await store().rename({ kind: 'folder', id: 'Auth' }, 'Authentification');
    await store().duplicate({ kind: 'folder', id: 'Auth' }, 'Auth (copie)');
    http.contentsAnswer = { folders: 1, requests: 2 };
    expect(await store().contents({ kind: 'folder', id: 'Factures' })).toEqual({ folders: 1, requests: 2 });

    http.failNext = new Error('disk');
    await store().createCollection('Perdue');

    expect(http.callsOf('rename').at(-1)).toEqual([{ kind: 'folder', id: 'Auth' }, 'Authentification']);
    expect(http.callsOf('duplicate')).toEqual([[{ kind: 'folder', id: 'Auth' }, 'Auth (copie)']]);
    expect(TestBed.inject(ErrorNotifier).notice()).not.toBeNull();
  });
});
