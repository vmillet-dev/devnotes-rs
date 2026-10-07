import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { EMPTY_PARTS } from '@core/model/http.model';
import { StatusNotifier } from '@core/services/notifications/status.service';
import { FakeHttpRepository } from '@testing/fake-http-repository';
import { historyEntry, historyItem } from '@testing/http-history.fixture';
import { provideAppTesting } from '@testing/testing.providers';
import { HttpHistoryStore } from './http-history.store';
import { HttpSendStore } from './http-send.store';
import { HttpTabsStore } from './http-tabs.store';

describe('HttpHistoryStore', () => {
  let http: FakeHttpRepository;

  const store = () => TestBed.inject(HttpHistoryStore);

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

  it('reopens an entry as a new draft tab, and steps aside for it', async () => {
    http.draftAnswer = {
      name: 'Payer',
      method: 'POST',
      document: { url: 'https://api.exemple.fr/pay', description: '' },
    };
    store().open();

    await store().reopen('Payer');

    const tab = TestBed.inject(HttpTabsStore).active();
    expect(tab?.requestId).toBeNull();
    expect(tab?.draft).toMatchObject({
      name: 'Payer',
      method: 'POST',
      parts: { url: 'https://api.exemple.fr/pay' },
    });
    expect(store().isOpen()).toBe(false);
  });
});
