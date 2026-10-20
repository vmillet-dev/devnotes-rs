import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { FakeHttpRepository } from '@testing/fake-http-repository';
import { provideAppTesting } from '@testing/testing.providers';
import { HttpCollectionsStore } from './http-collections.store';
import { HttpScriptFocusStore } from './http-script-focus.store';

describe('HttpScriptFocusStore', () => {
  let http: FakeHttpRepository;

  const focus = () => TestBed.inject(HttpScriptFocusStore);
  const collections = () => TestBed.inject(HttpCollectionsStore);

  beforeEach(async () => {
    TestBed.resetTestingModule();
    http = new FakeHttpRepository();
    http.tree$ = {
      collections: [
        {
          collection: {
            id: 'api',
            name: 'API Paiements',
            position: 0,
            createdAt: '2026-10-19T10:00:00.000Z',
          },
          children: [
            {
              kind: 'folder',
              folder: { id: 'invoices', collectionId: 'api', parentId: null, name: 'Factures', position: 0 },
              children: [],
            },
          ],
        },
      ],
    };
    TestBed.configureTestingModule({ providers: [provideAppTesting({ httpRepository: http })] });
    await collections().load();
  });

  it('leads a request’s own script to its tab, on the line, until the tab takes it', () => {
    focus().open('tab-1', { kind: 'error', level: 'request', script: 'after', line: 4, message: 'x' });

    expect(focus().request()).toEqual({ key: 'tab-1', focus: { phase: 'after', line: 4 } });
    expect(collections().settingsOpen()).toBeNull();
    focus().taken();
    expect(focus().request()).toBeNull();
  });

  it('opens a folder’s or the collection’s settings on their script', () => {
    focus().open('tab-1', {
      kind: 'error',
      level: 'folder',
      script: 'before',
      owner: 'invoices',
      line: 2,
      message: 'x',
    });

    const opening = collections().settingsOpen();
    expect(opening?.row.id).toBe('invoices');
    expect(opening?.row.kind).toBe('folder');
    expect(opening?.script).toEqual({ phase: 'before', line: 2 });
    expect(focus().request()).toBeNull();

    const revision = collections().settingsRevision();
    collections().closeSettings();
    expect(collections().settingsOpen()).toBeNull();
    expect(collections().settingsRevision()).toBe(revision + 1);
  });

  it('falls back on the request’s tab when what held the script is gone', () => {
    focus().open('tab-1', {
      kind: 'host',
      level: 'collection',
      script: 'before',
      owner: 'gone',
      message: 'x',
    });

    expect(collections().settingsOpen()).toBeNull();
    expect(focus().request()).toEqual({ key: 'tab-1', focus: { phase: 'before', line: null } });
  });
});
