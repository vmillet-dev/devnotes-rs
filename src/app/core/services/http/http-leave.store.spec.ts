import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { EVENT_SUBSCRIBER, GlobalAction } from '@core/ipc/app-events.service';
import { FakeAppWindow } from '@testing/fake-app-window';
import { FakeHttpRepository } from '@testing/fake-http-repository';
import { sampleTree } from '@testing/http-tree.fixture';
import { provideAppTesting } from '@testing/testing.providers';
import { HttpCollectionsStore } from './http-collections.store';
import { HttpLeaveStore } from './http-leave.store';
import { HttpSocketsStore } from './http-sockets.store';
import { HttpTabsStore } from './http-tabs.store';

/**
 * A request typed in a tab and never saved, or a saved one with changes, was gone after the
 * application was quit and started again — and nothing said so.
 */
describe('HttpLeaveStore', () => {
  let http: FakeHttpRepository;
  let appWindow: FakeAppWindow;
  let native: (action: GlobalAction) => void;

  const leave = () => TestBed.inject(HttpLeaveStore);
  const tabs = () => TestBed.inject(HttpTabsStore);
  const names = () =>
    leave()
      .modified()
      .map((tab) => tab.draft.name);
  /** Lets the promise chain of a click run: the stores await the fake, which answers at once. */
  const settled = () => new Promise<void>((resolve) => setTimeout(resolve));

  beforeEach(async () => {
    TestBed.resetTestingModule();
    http = new FakeHttpRepository();
    http.tree$ = sampleTree();
    http.seed('Login', {}, 'API', 'Auth');
    http.seed('Logout', {}, 'API', 'Auth');
    appWindow = new FakeAppWindow();
    native = () => undefined;
    TestBed.configureTestingModule({
      providers: [
        provideAppTesting({ httpRepository: http, appWindow }),
        {
          provide: EVENT_SUBSCRIBER,
          useValue: async (handler: (action: GlobalAction) => void) => {
            native = handler;
            return () => undefined;
          },
        },
      ],
    });
    await TestBed.inject(HttpCollectionsStore).load();
  });

  it('asks nothing when no tab is modified: leaving stays instant', async () => {
    await tabs().open('Login');

    expect(await leave().confirm('quit')).toBe(true);
    expect(leave().asking()).toBeNull();
  });

  it('counts a request never saved and a saved one with changes, not one saved a moment ago', async () => {
    await tabs().open('Login');
    await tabs().open('Logout');
    tabs().edit('Login', { name: 'Se connecter' });
    tabs().edit('Logout', { name: 'Sortir' });
    tabs().newRequest('graphql');
    tabs().edit(tabs().activeKey()!, { name: 'Brouillon' });
    expect(names()).toEqual(['Se connecter', 'Sortir', 'Brouillon']);

    await tabs().save('Logout');
    expect(names()).toEqual(['Se connecter', 'Brouillon']);
  });

  it('holds whoever asked until it is answered: go on without saving, or stay', async () => {
    await tabs().open('Login');
    tabs().edit('Login', { name: 'Se connecter' });

    const quitting = leave().confirm('quit');
    expect(leave().asking()).toBe('quit');
    leave().discard();
    expect(await quitting).toBe(true);
    expect(leave().asking()).toBeNull();
    // Nothing was written on the way out.
    expect(http.callsOf('saveRequest')).toEqual([]);

    const switching = leave().confirm('switch');
    expect(leave().asking()).toBe('switch');
    leave().cancel();
    expect(await switching).toBe(false);
  });

  it('keeps one question at a time: the one asked before is answered « stay »', async () => {
    tabs().newRequest();

    const first = leave().confirm('switch');
    const second = leave().confirm('quit');

    expect(await first).toBe(false);
    expect(leave().asking()).toBe('quit');
    leave().discard();
    expect(await second).toBe(true);
  });

  it('saves every request that has a place, then goes on', async () => {
    await tabs().open('Login');
    await tabs().open('Logout');
    tabs().edit('Login', { name: 'Se connecter' });
    tabs().edit('Logout', { name: 'Sortir' });

    const going = leave().confirm('update');
    await leave().saveAll();

    expect(await going).toBe(true);
    expect(http.callsOf('saveRequest').map((call) => call[0])).toEqual(['Login', 'Logout']);
    expect(names()).toEqual([]);
  });

  /** A request never saved has no collection: « Tout enregistrer » asks where each one goes. */
  it('walks the requests never saved through the save dialog, one at a time', async () => {
    await tabs().open('Login');
    tabs().edit('Login', { name: 'Se connecter' });
    tabs().newRequest();
    tabs().newRequest('websocket');
    const [, first, second] = tabs().tabs();

    let gone: boolean | null = null;
    void leave()
      .confirm('quit')
      .then((go) => (gone = go));
    await leave().saveAll();

    expect(http.callsOf('saveRequest')).toHaveLength(1);
    expect(leave().placing()?.key).toBe(first!.key);
    expect(gone).toBeNull();

    await leave().place({ collectionId: 'API', folderId: null }, 'Santé');
    expect(http.callsOf('createRequest').at(-1)?.[0]).toMatchObject({ name: 'Santé', collectionId: 'API' });
    expect(leave().placing()?.key).toBe(second!.key);

    await leave().place({ collectionId: '', folderId: null, create: { collection: 'Flux' } }, 'Live');
    expect(http.callsOf('createRequestIn').at(-1)).toMatchObject([{ name: 'Live', kind: 'websocket' }, {}]);
    await settled();
    expect(leave().placing()).toBeNull();
    expect(gone).toBe(true);
  });

  it('comes back to the question when a place is not given: that request would still be lost', async () => {
    tabs().newRequest();
    let gone: boolean | null = null;
    void leave()
      .confirm('quit')
      .then((go) => (gone = go));

    await leave().saveAll();
    expect(leave().placing()).not.toBeNull();
    leave().cancelPlace();

    expect(leave().placing()).toBeNull();
    expect(leave().asking()).toBe('quit');
    await settled();
    expect(gone).toBeNull();
  });

  it('stays on the question when a write fails, with what is left to save', async () => {
    await tabs().open('Login');
    tabs().edit('Login', { name: 'Se connecter' });
    let gone: boolean | null = null;
    void leave()
      .confirm('restore')
      .then((go) => (gone = go));

    http.failNext = new Error('disk');
    await leave().saveAll();

    await settled();
    expect(gone).toBeNull();
    expect(leave().asking()).toBe('restore');
    expect(names()).toEqual(['Se connecter']);
  });

  it('says how many sockets leaving cuts', async () => {
    http.seed('Live', { kind: 'websocket' }, 'Flux');
    await tabs().open('Live');
    expect(leave().connected()).toBe(0);

    await TestBed.inject(HttpSocketsStore).connect(tabs().active()!);
    expect(leave().connected()).toBe(1);
  });

  /**
   * The tray's « Quitter » and the window's close button ended the process on the spot. They
   * ask the page, which answers: at once with nothing to lose, after its question otherwise.
   */
  describe('a quit asked from outside the page', () => {
    beforeEach(() => {
      leave();
    });

    it('is done at once when nothing would be lost, without holding it', async () => {
      native('quit');
      await settled();

      expect(appWindow.quitsHeld).toBe(0);
      expect(appWindow.exitedWith).toBe(0);
    });

    it('is held while the question is out — the window shown — and ended by the answer', async () => {
      tabs().newRequest();

      native('quit');
      await settled();
      expect(appWindow.quitsHeld).toBe(1);
      expect(leave().asking()).toBe('quit');
      expect(appWindow.exitedWith).toBeNull();

      leave().discard();
      await settled();
      expect(appWindow.exitedWith).toBe(0);
    });

    it('is called off by « Annuler »: the application stays, and can be asked again', async () => {
      tabs().newRequest();

      native('quit');
      await settled();
      leave().cancel();
      await settled();
      expect(appWindow.exitedWith).toBeNull();

      native('quit');
      await settled();
      expect(appWindow.quitsHeld).toBe(2);
      expect(leave().asking()).toBe('quit');
    });

    it('leaves the other actions to the page they belong to', async () => {
      tabs().newRequest();
      native('palette');
      await settled();

      expect(leave().asking()).toBeNull();
      expect(appWindow.quitsHeld).toBe(0);
    });
  });
});
