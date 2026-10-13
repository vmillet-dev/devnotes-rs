import { TestBed } from '@angular/core/testing';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { EMPTY_PARTS } from '@core/model/http.model';
import { LibraryPreferencesService } from '@core/services/preferences/library-preferences.service';
import {
  FakeHttpEnvironmentsRepository,
  environment,
  variable,
} from '@testing/fake-http-environments-repository';
import { FakeHttpRepository } from '@testing/fake-http-repository';
import { provideAppTesting } from '@testing/testing.providers';
import { HttpEnvironmentsStore, VARIABLES_SAVE_MS } from './http-environments.store';
import { HttpSendStore } from './http-send.store';
import { HttpSocketsStore } from './http-sockets.store';
import { HttpWorkspaceStore } from './http-workspace.store';

const ACTIVE_KEY = 'devnotes.notes.http.environment';

describe('HttpEnvironmentsStore', () => {
  let environments: FakeHttpEnvironmentsRepository;
  let http: FakeHttpRepository;

  const store = () => TestBed.inject(HttpEnvironmentsStore);
  const preferences = () => TestBed.inject(LibraryPreferencesService);

  beforeEach(() => {
    TestBed.resetTestingModule();
    environments = new FakeHttpEnvironmentsRepository({
      globals: [variable('tenant', 'acme')],
      environments: [environment('Local'), environment('Production', { protected: true, colour: 'red' })],
    });
    http = new FakeHttpRepository();
    TestBed.configureTestingModule({
      providers: [provideAppTesting({ httpEnvironmentsRepository: environments, httpRepository: http })],
    });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('reads the globals and every environment', async () => {
    expect(await store().load()).toBe(true);

    expect(store().globals()).toEqual([variable('tenant', 'acme')]);
    expect(
      store()
        .environments()
        .map((held) => held.name),
    ).toEqual(['Local', 'Production']);
  });

  it('remembers the environment chosen with the library, and none by default', async () => {
    await store().load();
    expect(store().active()).toBeNull();

    store().choose('env-local');

    expect(store().active()?.name).toBe('Local');
    expect(preferences().read(ACTIVE_KEY)).toBe('env-local');
    store().choose(null);
    expect(preferences().read(ACTIVE_KEY)).toBe('');
  });

  it('reads the one chosen at the last launch', async () => {
    preferences().write(ACTIVE_KEY, 'env-production');
    await store().load();

    expect(store().active()?.name).toBe('Production');
  });

  it('sends with none when the one remembered is gone', async () => {
    preferences().write(ACTIVE_KEY, 'env-gone');
    await store().load();

    expect(store().active()).toBeNull();
  });

  it('shows the page in place of the requests, and leaves it by the same button', () => {
    store().toggle();
    expect(store().isOpen()).toBe(true);
    expect(TestBed.inject(HttpWorkspaceStore).page()).toBe('environments');

    store().toggle();
    expect(store().isOpen()).toBe(false);
  });

  it('opens on what it creates', async () => {
    await store().load();

    expect(await store().create('Staging')).toBe(true);

    expect(
      store()
        .environments()
        .map((held) => held.name),
    ).toEqual(['Local', 'Production', 'Staging']);
    expect(store().shown()?.name).toBe('Staging');
  });

  it('keeps the page as it was when Rust refuses a name', async () => {
    await store().load();
    environments.failNext = new Error('duplicate');

    expect(await store().create('local')).toBe(false);
    expect(store().environments()).toHaveLength(2);
  });

  it('writes a table once the keys rest, and shows it as typed meanwhile', async () => {
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
    await store().load();
    store().select({ kind: 'environment', id: 'env-local' });

    store().editVariables([variable('baseUrl', 'http://localhost')]);
    store().editVariables([variable('baseUrl', 'http://localhost:8080')]);

    expect(store().rows()).toEqual([variable('baseUrl', 'http://localhost:8080')]);
    expect(environments.callsTo('save')).toEqual([]);
    await vi.advanceTimersByTimeAsync(VARIABLES_SAVE_MS);

    expect(environments.callsTo('save')).toEqual([
      { command: 'save', args: ['env-local', { variables: [variable('baseUrl', 'http://localhost:8080')] }] },
    ]);
    expect(store().shown()?.variables).toEqual([variable('baseUrl', 'http://localhost:8080')]);
  });

  it('writes what waits before another set is shown', async () => {
    await store().load();
    store().editVariables([variable('tenant', 'globex')]);

    store().select({ kind: 'environment', id: 'env-local' });

    expect(environments.callsTo('saveGlobals')).toHaveLength(1);
    await vi.waitFor(() => expect(store().globals()).toEqual([variable('tenant', 'globex')]));
  });

  it('puts the copy right after its original and shows it', async () => {
    await store().load();

    await store().duplicate('env-local', 'Local (copie)');

    expect(
      store()
        .environments()
        .map((held) => held.name),
    ).toEqual(['Local', 'Local (copie)', 'Production']);
    expect(store().shown()?.name).toBe('Local (copie)');
  });

  it('forgets the active environment when it is deleted, and shows the globals', async () => {
    await store().load();
    store().choose('env-local');
    store().select({ kind: 'environment', id: 'env-local' });

    await store().delete('env-local');

    expect(store().active()).toBeNull();
    expect(store().target()).toEqual({ kind: 'globals' });
    expect(
      store()
        .environments()
        .map((held) => held.name),
    ).toEqual(['Production']);
  });

  describe('under a protected environment', () => {
    const tab = (method: 'GET' | 'POST' | 'HEAD' | 'OPTIONS' | 'DELETE') => ({
      key: 'tab-1',
      place: null,
      requestId: null,
      draft: { name: 'Créer', kind: 'http' as const, method, parts: EMPTY_PARTS },
    });

    beforeEach(async () => {
      await store().load();
      store().choose('env-production');
    });

    it('sends what reads without asking', async () => {
      for (const method of ['GET', 'HEAD', 'OPTIONS'] as const) {
        await TestBed.inject(HttpSendStore).send(tab(method));
      }

      expect(http.callsOf('send')).toHaveLength(3);
      expect(store().asking()).toBeNull();
    });

    it('asks before anything else, and sends on the yes', async () => {
      const sending = TestBed.inject(HttpSendStore);
      await sending.send(tab('POST'));

      expect(http.callsOf('send')).toEqual([]);
      expect(store().asking()).toBe('tab-1');

      await sending.send(tab('POST'), true);
      expect(http.callsOf('send')).toHaveLength(1);
      expect(store().asking()).toBeNull();
    });

    it('asks before a socket opens', async () => {
      await TestBed.inject(HttpSocketsStore).connect({
        key: 'tab-1',
        place: null,
        draft: { parts: EMPTY_PARTS },
      });

      expect(http.callsOf('connectWebsocket')).toEqual([]);
      expect(store().asking()).toBe('tab-1');
    });

    it('stops asking when another environment is chosen', async () => {
      await TestBed.inject(HttpSendStore).send(tab('DELETE'));

      store().choose('env-local');

      expect(store().asking()).toBeNull();
      await TestBed.inject(HttpSendStore).send(tab('DELETE'));
      expect(http.callsOf('send')).toHaveLength(1);
    });
  });
});
