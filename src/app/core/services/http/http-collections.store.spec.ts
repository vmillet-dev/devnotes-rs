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

  it('creates under a parent it unfolds, reads the tree again, and opens a new request', async () => {
    await store().load();
    store().toggle('Auth');

    await store().createFolder('API', null, 'Webhooks');
    await store().createRequest('API', 'Auth', 'Refresh');

    expect(http.callsOf('createFolder')).toEqual([['API', null, 'Webhooks']]);
    expect(http.callsOf('createRequest')[0]?.[0]).toMatchObject({
      collectionId: 'API',
      folderId: 'Auth',
      name: 'Refresh',
      kind: 'http',
      method: 'GET',
    });
    expect(store().collapsed().has('Auth')).toBe(false);
    expect(store().openId()).toMatch(/^request-/);
    expect(http.callsOf('tree').length).toBeGreaterThanOrEqual(3);
  });

  it('closes the open request when a deletion took it, and keeps it otherwise', async () => {
    await store().load();
    store().open('Old');
    await store().delete({ kind: 'request', id: 'Health' });
    expect(store().openId()).toBe('Old');

    http.tree$ = { collections: [] };
    await store().delete({ kind: 'collection', id: 'API' });
    expect(store().openId()).toBeNull();
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

  it('renames, copies and counts through Rust, and says when a write failed', async () => {
    await store().rename({ kind: 'folder', id: 'Auth' }, 'Authentification');
    await store().duplicate({ kind: 'folder', id: 'Auth' }, 'Auth (copie)');
    http.contentsAnswer = { folders: 1, requests: 2 };
    expect(await store().contents({ kind: 'folder', id: 'Factures' })).toEqual({ folders: 1, requests: 2 });

    http.failNext = new Error('disk');
    await store().createCollection('Perdue');

    expect(http.callsOf('rename')).toEqual([[{ kind: 'folder', id: 'Auth' }, 'Authentification']]);
    expect(http.callsOf('duplicate')).toEqual([[{ kind: 'folder', id: 'Auth' }, 'Auth (copie)']]);
    expect(TestBed.inject(ErrorNotifier).notice()).not.toBeNull();
  });
});
