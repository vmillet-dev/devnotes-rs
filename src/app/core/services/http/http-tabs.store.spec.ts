import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { LibraryPreferencesService } from '@core/services/preferences/library-preferences.service';
import { EMPTY_PARTS, HttpTree } from '@core/model/http.model';
import { FakeHttpRepository } from '@testing/fake-http-repository';
import { TranslocoService } from '@jsverse/transloco';
import { firstValueFrom } from 'rxjs';
import { collection, folder, request } from '@testing/http-tree.fixture';
import { provideAppTesting } from '@testing/testing.providers';
import { HttpCollectionsStore } from './http-collections.store';
import { HttpTabsStore } from './http-tabs.store';

const KEY = 'devnotes.notes.http.tabs';

describe('HttpTabsStore', () => {
  let http: FakeHttpRepository;

  const store = () => TestBed.inject(HttpTabsStore);
  const collections = () => TestBed.inject(HttpCollectionsStore);

  /** API: Login (named as told, at the root or in `Auth`), Health. What Rust answers after a rail's write. */
  const treeWith = (login: { name?: string; folderId?: string | null } | null): HttpTree => {
    const node = request('Login', 'API', login?.folderId ?? null, 'POST');
    if (node.kind === 'request') node.request.name = login?.name ?? 'Login';
    const held = login === null ? [] : [node];
    return {
      collections: [
        collection('API', [
          folder('Auth', 'API', login?.folderId === 'Auth' ? held : []),
          ...(login?.folderId === 'Auth' ? [] : held),
          request('Health', 'API'),
        ]),
      ],
    };
  };

  /** The tree as it reads after a write of the rail, and the tabs once they followed it. */
  const read = async (tree: HttpTree) => {
    http.tree$ = tree;
    await collections().load();
    TestBed.tick();
  };
  const stored = () => JSON.parse(TestBed.inject(LibraryPreferencesService).read(KEY) ?? 'null');

  beforeEach(() => {
    TestBed.resetTestingModule();
    http = new FakeHttpRepository();
    http.seed('Login', {
      method: 'POST',
      parts: { ...EMPTY_PARTS, url: '/login' },
    });
    http.seed('Health');
    TestBed.configureTestingModule({ providers: [provideAppTesting({ httpRepository: http })] });
  });

  it('opens a request once, in a tab of its own, and remembers which are open', async () => {
    await store().open('Login');
    await store().open('Health');
    await store().open('Login');

    expect(
      store()
        .tabs()
        .map((tab) => tab.key),
    ).toEqual(['Login', 'Health']);
    expect(store().activeRequestId()).toBe('Login');
    expect(http.callsOf('request')).toHaveLength(2);
    expect(stored()).toEqual({ open: ['Login', 'Health'], active: 'Login' });
  });

  it('marks a tab modified until it is saved, and saves it whole', async () => {
    await store().open('Login');
    const key = store().activeKey()!;
    expect(store().isDirty(store().active()!)).toBe(false);

    store().edit(key, { method: 'PUT', name: 'Se connecter' });
    store().editParts(key, { headers: [{ enabled: true, key: 'Accept', value: '*/*', description: '' }] });
    expect(store().isDirty(store().active()!)).toBe(true);

    expect(await store().save(key)).toBe('Login');
    expect(http.callsOf('saveRequest')[0]?.[1]).toMatchObject({ name: 'Se connecter', method: 'PUT' });
    expect(store().isDirty(store().active()!)).toBe(false);
  });

  /** A kind left is set aside, not emptied: it is how a body leaves a `GET` without being lost. */
  it('keeps what each kind of body held while its tab is open, and gives it back', async () => {
    await store().open('Login');
    const key = store().activeKey()!;
    const body = () => store().active()!.draft.parts.body;

    store().chooseBodyKind(key, 'json');
    store().editParts(key, { body: { kind: 'json', text: '{"q":"devnotes"}' } });
    store().chooseBodyKind(key, 'none');
    expect(body()).toEqual({ kind: 'none' });

    store().chooseBodyKind(key, 'json');
    expect(body()).toEqual({ kind: 'json', text: '{"q":"devnotes"}' });
    // The same kind again changes nothing; an unknown tab neither.
    store().chooseBodyKind(key, 'json');
    store().chooseBodyKind('nowhere', 'form');
    expect(body()).toEqual({ kind: 'json', text: '{"q":"devnotes"}' });

    // Not another tab's, and not once the tab was closed.
    await store().open('Health');
    store().chooseBodyKind('Health', 'json');
    expect(body()).toEqual({ kind: 'json', text: '' });
    store().close('Login');
    await store().open('Login');
    store().chooseBodyKind('Login', 'json');
    expect(body()).toEqual({ kind: 'json', text: '' });
  });

  /** GraphQL and WebSocket were the last two entries of the method menu, and were not found. */
  it('begins a request as the kind asked, HTTP unless told, and saves it as that kind', async () => {
    store().newRequest();
    store().newRequest('graphql');
    store().newRequest('websocket');
    expect(
      store()
        .tabs()
        .map((tab) => tab.draft.kind),
    ).toEqual(['http', 'graphql', 'websocket']);

    await store().save(store().activeKey()!, { collectionId: 'API', folderId: null });
    expect(http.callsOf('createRequest').at(-1)?.[0]).toMatchObject({ kind: 'websocket' });
  });

  /**
   * On a library with no collection the first save needed a trip to the rail. The save hands
   * Rust the request **and** what it lacks, in one call: a request that is refused leaves no
   * empty collection behind it.
   */
  it('saves a new request with the collection or the folder it lacks, in one write', async () => {
    store().newRequest();
    const key = store().activeKey()!;
    store().edit(key, { name: 'Santé' });

    await store().save(key, { collectionId: '', folderId: null, create: { collection: 'Boutique' } });

    expect(http.callsOf('createCollection')).toEqual([]);
    expect(http.callsOf('createRequest')).toEqual([]);
    expect(http.callsOf('createRequestIn')).toHaveLength(1);
    expect(http.callsOf('createRequestIn')[0]).toMatchObject([
      { name: 'Santé', kind: 'http', method: 'GET' },
      { collection: 'Boutique' },
    ]);

    store().newRequest('graphql');
    await store().save(store().activeKey()!, {
      collectionId: 'API',
      folderId: 'Auth',
      create: { folder: 'Jetons' },
    });
    expect(http.callsOf('createRequestIn')[1]).toMatchObject([
      { collectionId: 'API', folderId: 'Auth', kind: 'graphql' },
      { folder: 'Jetons' },
    ]);
    expect(http.callsOf('createFolder')).toEqual([]);
  });

  it('takes the place Rust answers, not the one it asked for, and says when the write failed', async () => {
    store().newRequest();
    const key = store().activeKey()!;
    http.failNext = new Error('disk');
    expect(
      await store().save(key, { collectionId: '', folderId: null, create: { collection: 'X' } }),
    ).toBeNull();
    expect(store().tabs()[0]).toMatchObject({ key, requestId: null, place: null });

    // The tree the save reads back holds the request: its tab stays, placed where Rust put it.
    http.tree$ = {
      collections: [
        collection('collection-1', [
          folder('folder-2', 'collection-1', [request('request-3', 'collection-1', 'folder-2')]),
        ]),
      ],
    };
    const saved = await store().save(key, {
      collectionId: '',
      folderId: null,
      create: { collection: 'Boutique', folder: 'Commandes' },
    });

    expect(saved).toBe('request-3');
    expect(store().active()).toMatchObject({
      requestId: 'request-3',
      place: { collectionId: 'collection-1', folderId: 'folder-2' },
    });
  });

  /** A tab closed by a deletion took what was typed in it: the rail asks for these first. */
  it('names the open tabs with unsaved changes among the requests a deletion takes', async () => {
    http.seed('Logout');
    await store().open('Login');
    await store().open('Logout');
    const among = (...ids: string[]) => store().unsavedAmong(new Set(ids));
    expect(among('Login', 'Logout')).toEqual([]);

    store().edit('Login', { name: 'Se connecter' });
    expect(among('Login', 'Logout')).toEqual(['Se connecter']);
    expect(among('Logout')).toEqual([]);

    // A request never saved is in no folder: no deletion takes it.
    store().newRequest();
    expect(among('Login', 'Logout')).toEqual(['Se connecter']);

    await store().save('Login');
    expect(among('Login', 'Logout')).toEqual([]);
  });

  it('keeps the bodies set aside when a request never saved takes its id', async () => {
    store().newRequest();
    const draft = store().activeKey()!;
    store().chooseBodyKind(draft, 'text');
    store().editParts(draft, { body: { kind: 'text', text: 'bonjour' } });
    store().chooseBodyKind(draft, 'none');

    const key = (await store().save(draft, { collectionId: 'API', folderId: null }))!;
    store().chooseBodyKind(key, 'text');

    expect(store().active()!.draft.parts.body).toEqual({ kind: 'text', text: 'bonjour' });
  });

  it('rewrites the table from a typed URL, and the URL from the table, through Rust', async () => {
    await store().open('Login');
    const key = store().activeKey()!;

    http.synced = { url: '/login?a=1', params: [{ enabled: true, key: 'a', value: '1', description: '' }] };
    await store().editUrl(key, '/login?a=1');
    expect(store().active()!.draft.parts.params).toEqual(http.synced.params);
    expect(http.callsOf('syncQuery')[0]?.[2]).toBe('url');

    const params = [{ enabled: true, key: 'b', value: '2', description: '' }];
    http.synced = { url: '/login?b=2', params };
    await store().editParams(key, params);
    expect(store().active()!.draft.parts.url).toBe('/login?b=2');
  });

  it('keeps a new request a draft until it is placed, then gives its tab the request’s id', async () => {
    await firstValueFrom(TestBed.inject(TranslocoService).load('fr'));
    store().newRequest();
    const key = store().activeKey()!;
    expect(key).toMatch(/^draft-/);
    expect(store().active()!.draft.name).toBe('Nouvelle requête');
    expect(store().isDirty(store().active()!)).toBe(true);

    expect(await store().save(key)).toBeNull();
    expect(stored()).toEqual({ open: [], active: null });

    const saved = await store().save(key, { collectionId: 'API', folderId: 'Auth' });
    expect(saved).toMatch(/^request-/);
    expect(store().activeKey()).toBe(saved);
    expect(http.callsOf('createRequest')[0]?.[0]).toMatchObject({ collectionId: 'API', folderId: 'Auth' });
  });

  it('closes a tab for its neighbour, and those of requests the tree no longer holds', async () => {
    await read(treeWith({}));
    await store().open('Login');
    await store().open('Health');
    store().close('Health');
    expect(store().activeKey()).toBe('Login');

    await read(treeWith(null));
    expect(store().tabs()).toEqual([]);
    expect(store().active()).toBeNull();
  });

  /** An empty tree before a first read is no deletion; a draft never saved is in no tree. */
  it('closes nothing before the tree was read, and never a request that was not saved yet', async () => {
    await store().open('Login');
    store().newRequest();
    TestBed.tick();
    expect(store().tabs()).toHaveLength(2);

    await read(treeWith({}));
    expect(
      store()
        .tabs()
        .map((tab) => tab.requestId),
    ).toEqual(['Login', null]);
  });

  /**
   * A request's name lived in two places. Renamed from the rail, its tab kept the name it was
   * opened with — and the next save wrote the whole draft, old name included, over the new one.
   */
  it('takes the name the rail gave an open request, and saves that one', async () => {
    await read(treeWith({}));
    await store().open('Login');
    const key = store().activeKey()!;
    store().editParts(key, { description: 'en cours' });

    await read(treeWith({ name: 'Connexion' }));

    const tab = store().active()!;
    expect([tab.saved?.name, tab.draft.name]).toEqual(['Connexion', 'Connexion']);
    // What was being edited is still there, and still to save.
    expect(tab.draft.parts.description).toBe('en cours');
    expect(store().isDirty(tab)).toBe(true);

    await store().save(key);
    expect(http.callsOf('saveRequest').at(-1)?.[1]).toMatchObject({ name: 'Connexion' });
  });

  it('leaves an unmodified tab unmodified by a rename from the rail', async () => {
    await read(treeWith({}));
    await store().open('Login');

    await read(treeWith({ name: 'Connexion' }));

    expect(store().active()!.draft.name).toBe('Connexion');
    expect(store().isDirty(store().active()!)).toBe(false);
  });

  /** The other direction: the rail shows the name and the method, and reads what a save wrote. */
  it('has the tree read again once a tab is saved, an existing request or a new one', async () => {
    await read(treeWith({}));
    await store().open('Login');
    const reads = () => http.callsOf('tree').length;
    const before = reads();

    store().edit('Login', { name: 'Se connecter' });
    await store().save('Login');
    expect(reads()).toBe(before + 1);

    store().newRequest();
    await store().save(store().activeKey()!, { collectionId: 'API', folderId: null });
    expect(reads()).toBe(before + 2);
  });

  /** Moved by the rail, it inherits from where it now sits — not from the folder it was opened in. */
  it('follows an open request moved to another folder', async () => {
    await read(treeWith({}));
    await store().open('Login');
    expect(store().active()!.place).toEqual({ collectionId: 'API', folderId: null });

    await read(treeWith({ folderId: 'Auth' }));

    expect(store().active()!.place).toEqual({ collectionId: 'API', folderId: 'Auth' });
    expect(store().isDirty(store().active()!)).toBe(false);
  });

  it('reopens what was open last time, skipping what is gone or was written by hand', async () => {
    TestBed.inject(LibraryPreferencesService).write(
      KEY,
      JSON.stringify({ open: ['Login', 'Gone', 7], active: 'Login' }),
    );
    await store().restore();
    await store().restore();

    expect(
      store()
        .tabs()
        .map((tab) => tab.key),
    ).toEqual(['Login']);
    expect(store().activeKey()).toBe('Login');
    expect(http.callsOf('request')).toHaveLength(2);

    TestBed.resetTestingModule();
    TestBed.configureTestingModule({ providers: [provideAppTesting({ httpRepository: http })] });
    TestBed.inject(LibraryPreferencesService).write(KEY, '{broken');
    await store().restore();
    expect(store().tabs()).toEqual([]);
  });

  it('says when a request will not open or save', async () => {
    http.failNext = new Error('disk');
    await store().open('Login');
    expect(store().tabs()).toEqual([]);
    expect(TestBed.inject(ErrorNotifier).notice()).not.toBeNull();

    await store().open('Login');
    http.failNext = new Error('disk');
    expect(await store().save('Login')).toBeNull();
  });
});
