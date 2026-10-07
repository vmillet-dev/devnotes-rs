import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { IpcError } from '@core/ipc/ipc.error';
import { WEBSOCKET_SUBSCRIBER } from '@core/ipc/websocket-events';
import { EMPTY_PARTS, SocketEvent, WebsocketEvent } from '@core/model/http.model';
import { FakeHttpRepository } from '@testing/fake-http-repository';
import { provideAppTesting } from '@testing/testing.providers';
import { Connectable, HttpSocketsStore, LOG_KEPT } from './http-sockets.store';

const tab: Connectable = {
  key: 'Flux',
  place: { collectionId: 'API', folderId: null },
  draft: { parts: EMPTY_PARTS },
};

describe('HttpSocketsStore', () => {
  let http: FakeHttpRepository;
  let play: (event: WebsocketEvent) => void;

  const store = () => TestBed.inject(HttpSocketsStore);
  const state = () => store().states().get('Flux');
  const socketId = () => state()!.socketId;
  const event = (kind: SocketEvent) => play({ id: socketId(), at: '2026-10-07T12:00:00.000Z', event: kind });

  beforeEach(() => {
    TestBed.resetTestingModule();
    http = new FakeHttpRepository();
    TestBed.configureTestingModule({
      providers: [
        provideAppTesting({ httpRepository: http }),
        {
          provide: WEBSOCKET_SUBSCRIBER,
          useValue: (handler: (event: WebsocketEvent) => void) => {
            play = handler;
            return Promise.resolve(() => undefined);
          },
        },
      ],
    });
    store();
  });

  it('connects under an id of its own, then follows the socket by its events', async () => {
    await store().connect(tab);
    expect(http.callsOf('connectWebsocket')[0]).toEqual([socketId(), EMPTY_PARTS, tab.place]);
    expect(state()?.phase).toBe('connecting');

    event({ kind: 'opened', url: 'wss://a', protocol: 'chat' });
    expect(state()).toMatchObject({ phase: 'open', protocol: 'chat' });

    await store().send('Flux', 'hello');
    expect(http.callsOf('sendWebsocket')[0]).toEqual([socketId(), 'hello']);
    event({ kind: 'sent', text: 'hello', size: 5 });
    event({ kind: 'received', text: 'echo', size: 4, binary: false, cut: false });
    expect(state()!.log.map((entry) => entry.event.kind)).toEqual(['opened', 'sent', 'received']);

    await store().close('Flux');
    expect(state()?.phase).toBe('closing');
    event({ kind: 'closed', code: 1000, reason: '' });
    expect(state()?.phase).toBe('closed');
  });

  it('says why a socket would not open, or why it broke', async () => {
    http.failNext = new IpcError('connect_websocket', {
      code: 'httpWebsocketRefused',
      params: { status: '401' },
      detail: 'refused',
    });
    await store().connect(tab);
    expect(state()).toMatchObject({
      phase: 'closed',
      notice: { ref: { key: 'errors.httpWebsocketRefused', params: { status: '401' } } },
    });

    await store().connect(tab);
    event({ kind: 'opened', url: 'ws://a', protocol: null });
    event({ kind: 'failed', code: 'httpNetwork', detail: 'reset' });
    expect(state()).toMatchObject({ phase: 'closed', notice: { ref: { key: 'errors.httpNetwork' } } });
  });

  it('keeps the newest of a long log, follows a draft saved, and closes with its tab', async () => {
    await store().connect(tab);
    for (let count = 0; count < LOG_KEPT + 5; count++) {
      event({ kind: 'received', text: String(count), size: 1, binary: false, cut: false });
    }
    expect(state()!.log).toHaveLength(LOG_KEPT);
    expect(state()!.log.at(-1)?.event).toMatchObject({ text: String(LOG_KEPT + 4) });
    store().clearLog('Flux');
    expect(state()!.log).toEqual([]);

    const id = socketId();
    store().rekey('Flux', 'request-1');
    store().forget('request-1');
    expect(store().states().size).toBe(0);
    expect(http.callsOf('closeWebsocket')).toEqual([[id]]);
  });
});
