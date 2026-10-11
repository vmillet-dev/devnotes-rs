import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { EMPTY_PARTS, HttpTree } from '@core/model/http.model';
import { StatusNotifier } from '@core/services/notifications/status.service';
import { FakeHttpRepository } from '@testing/fake-http-repository';
import { historyEntry, historyItem } from '@testing/http-history.fixture';
import { collection, request } from '@testing/http-tree.fixture';
import { provideAppTesting } from '@testing/testing.providers';
import { HttpCollectionsStore } from './http-collections.store';
import { HttpHistoryStore } from './http-history.store';
import { HttpSendStore } from './http-send.store';
import { HttpTabsStore } from './http-tabs.store';

describe('HttpHistoryStore', () => {
  let http: FakeHttpRepository;

  const store = () => TestBed.inject(HttpHistoryStore);
  const tabs = () => TestBed.inject(HttpTabsStore);
  /** A send of `Login`, the request the tree holds, put back as Rust would answer it. */
  const sentFromLogin = async () => {
    const tree: HttpTree = { collections: [collection('API', [request('Login', 'API')])] };
    http.tree$ = tree;
    http.seed('Login', { parts: { ...EMPTY_PARTS, url: 'https://api.exemple.fr/now' } });
    await TestBed.inject(HttpCollectionsStore).load();
    http.ontoAnswer = {
      name: 'Login',
      method: 'POST',
      document: { url: 'https://api.exemple.fr/then', description: '' },
    };
    return historyEntry(historyItem('h1', { requestId: 'Login' }));
  };

  beforeEach(() => {
    TestBed.resetTestingModule();
    http = new FakeHttpRepository();
    http.historyAnswer = [{ day: '2026-10-07', items: [historyItem('Login'), historyItem('Health')] }];
    TestBed.configureTestingModule({ providers: [provideAppTesting({ httpRepository: http })] });
  });

  it('reads nothing until opened, then again after each send that ended', async () => {
    TestBed.tick();
    expect(http.callsOf('history')).toEqual([]);

    store().open();
    await vi.waitFor(() => expect(store().days()).toHaveLength(1));
    expect(http.callsOf('history')[0]).toEqual([new Date().getTimezoneOffset()]);

    await TestBed.inject(HttpSendStore).send({
      key: 'Login',
      place: null,
      requestId: null,
      draft: { name: 'Login', kind: 'http', method: 'GET', parts: EMPTY_PARTS },
    });
    await vi.waitFor(() => expect(http.callsOf('history')).toHaveLength(2));
  });

  it('opens the entry chosen', async () => {
    http.entries.set('Login', historyEntry(historyItem('Login')));
    store().open();
    store().select('Login');

    await vi.waitFor(() => expect(store().entry()?.item.id).toBe('Login'));
  });

  it('clears everything, says how much went, and forgets the entry chosen', async () => {
    store().open();
    store().select('Login');
    expect(await store().count()).toBe(2);

    await store().clear();

    expect(store().selectedId()).toBeNull();
    expect(TestBed.inject(StatusNotifier).status()).toEqual({
      key: 'http.history.cleared',
      params: { count: 2 },
    });
    await vi.waitFor(() => expect(store().days()).toEqual([]));
  });

  /** « Vider l'historique » was the only way out for the one send made with the wrong token. */
  it('deletes one send, offers to put it back for a while, and reads the list again', async () => {
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
    try {
      const login = historyEntry(historyItem('Login'));
      http.entries.set('Login', login);
      store().open();
      store().select('Login');
      const reads = () => http.callsOf('history').length;
      await vi.waitFor(() => expect(reads()).toBe(1));

      await store().delete('Login');
      expect(store().removed()).toBe(login);
      expect(store().selectedId()).toBeNull();
      await vi.waitFor(() => expect(reads()).toBe(2));

      await store().undoDelete();
      expect(http.callsOf('restoreHistoryEntry')).toEqual([[login]]);
      expect(store().removed()).toBeNull();
      await vi.waitFor(() => expect(reads()).toBe(3));

      // Left alone, the offer goes; dismissed, it goes at once. Nothing is put back.
      http.entries.set('Health', historyEntry(historyItem('Health')));
      await store().delete('Health');
      vi.advanceTimersByTime(8000);
      expect(store().removed()).toBeNull();
      await store().undoDelete();
      await store().delete('Login');
      store().dismissUndo();
      expect(store().removed()).toBeNull();
      expect(http.callsOf('restoreHistoryEntry')).toHaveLength(1);
    } finally {
      vi.useRealTimers();
    }
  });

  /** « Rouvrir dans un onglet » made a stranger of it: a draft with no tie to its request. */
  it('puts a send back onto the request it came from, as changes not saved', async () => {
    const entry = await sentFromLogin();
    store().open();
    expect(store().requestOf(entry.item)?.name).toBe('Login');

    await store().reopen(entry);

    const tab = tabs().active()!;
    expect(tab.requestId).toBe('Login');
    expect(tab.draft).toMatchObject({
      name: 'Login',
      method: 'POST',
      parts: { url: 'https://api.exemple.fr/then' },
    });
    expect(tab.saved?.parts.url).toBe('https://api.exemple.fr/now');
    expect(tabs().isDirty(tab)).toBe(true);
    expect(http.callsOf('historyOnto')).toEqual([['h1']]);
    expect(store().isOpen()).toBe(false);
  });

  it('asks before writing over the changes its open tab already holds', async () => {
    const entry = await sentFromLogin();
    await tabs().open('Login');
    tabs().editParts('Login', { description: 'en cours' });
    store().open();

    await store().reopen(entry);
    expect(store().replacing()).toEqual({ entryId: 'h1', name: 'Login' });
    expect(http.callsOf('historyOnto')).toEqual([]);
    expect(tabs().active()!.draft.parts.description).toBe('en cours');
    expect(store().isOpen()).toBe(true);

    store().cancelReplace();
    expect(store().replacing()).toBeNull();

    await store().reopen(entry);
    await store().reopen(entry, true);
    expect(store().replacing()).toBeNull();
    expect(tabs().active()!.draft.parts).toMatchObject({
      url: 'https://api.exemple.fr/then',
      description: '',
    });
    expect(store().isOpen()).toBe(false);
  });

  it('opens a draft named after the send when its request is gone, or when it came from none', async () => {
    http.draftAnswer = {
      name: 'Payer',
      method: 'POST',
      document: { url: 'https://api.exemple.fr/pay', description: '' },
    };
    const time = new Intl.DateTimeFormat('fr', { hour: '2-digit', minute: '2-digit' }).format(
      new Date('2026-10-07T12:00:00.000Z'),
    );
    store().open();

    // Sent from a draft: nothing to go back onto, and Rust is not asked.
    await store().reopen(historyEntry(historyItem('Payer')));
    expect(tabs().active()?.requestId).toBeNull();
    expect(tabs().active()?.draft).toMatchObject({
      name: `Payer · ${time}`,
      method: 'POST',
      parts: { url: 'https://api.exemple.fr/pay' },
    });
    expect(http.callsOf('historyOnto')).toEqual([]);
    expect(store().isOpen()).toBe(false);

    // In the tree a moment ago, gone when Rust looks: a draft too.
    const entry = await sentFromLogin();
    http.ontoAnswer = null;
    store().open();
    await store().reopen(entry);
    expect(http.callsOf('historyOnto')).toEqual([['h1']]);
    expect(
      tabs()
        .tabs()
        .map((tab) => tab.requestId),
    ).toEqual([null, null]);
  });
});
