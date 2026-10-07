import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { LibraryPreferencesService } from '@core/services/preferences/library-preferences.service';
import { FakeHttpRepository } from '@testing/fake-http-repository';
import { TranslocoService } from '@jsverse/transloco';
import { firstValueFrom } from 'rxjs';
import { provideAppTesting } from '@testing/testing.providers';
import { HttpTabsStore } from './http-tabs.store';

const KEY = 'devnotes.notes.http.tabs';

describe('HttpTabsStore', () => {
  let http: FakeHttpRepository;

  const store = () => TestBed.inject(HttpTabsStore);
  const stored = () => JSON.parse(TestBed.inject(LibraryPreferencesService).read(KEY) ?? 'null');

  beforeEach(() => {
    TestBed.resetTestingModule();
    http = new FakeHttpRepository();
    http.seed('Login', {
      method: 'POST',
      parts: { url: '/login', params: [], headers: [], description: '' },
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
    await store().open('Login');
    await store().open('Health');
    store().close('Health');
    expect(store().activeKey()).toBe('Login');

    store().prune({ collections: [] });
    expect(store().tabs()).toEqual([]);
    expect(store().active()).toBeNull();
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
