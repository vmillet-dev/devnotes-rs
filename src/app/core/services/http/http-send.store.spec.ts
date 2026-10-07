import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { IpcError } from '@core/ipc/ipc.error';
import { EMPTY_PARTS, RequestDraft, SentResponse } from '@core/model/http.model';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { StatusNotifier } from '@core/services/notifications/status.service';
import { FakeFileDialog } from '@testing/fake-file-dialog';
import { FakeHttpRepository, sentResponse } from '@testing/fake-http-repository';
import { provideAppTesting } from '@testing/testing.providers';
import { HttpSendStore, Sendable, responseFileName } from './http-send.store';

const draft: RequestDraft = { name: 'Login', kind: 'http', method: 'POST', parts: EMPTY_PARTS };
const tab = (key: string, place: Sendable['place'] = null): Sendable => ({
  key,
  place,
  draft,
  requestId: null,
});

function deferred<T>(): {
  promise: Promise<T>;
  resolve: (value: T) => void;
  reject: (cause: unknown) => void;
} {
  let resolve!: (value: T) => void;
  let reject!: (cause: unknown) => void;
  const promise = new Promise<T>((done, fail) => {
    resolve = done;
    reject = fail;
  });
  return { promise, resolve, reject };
}

describe('HttpSendStore', () => {
  let http: FakeHttpRepository;
  let dialog: FakeFileDialog;

  const store = () => TestBed.inject(HttpSendStore);
  const phase = (key: string) => store().states().get(key)?.phase;

  beforeEach(() => {
    TestBed.resetTestingModule();
    http = new FakeHttpRepository();
    dialog = new FakeFileDialog();
    TestBed.configureTestingModule({
      providers: [provideAppTesting({ httpRepository: http, fileDialog: dialog })],
    });
  });

  it('sends under an id of its own, with where the request sits, and keeps the answer', async () => {
    await store().send(tab('Login', { collectionId: 'API', folderId: 'Auth' }));

    const [id, sent, place] = http.callsOf('send')[0]!;
    expect(id).toMatch(/^Login#\d+$/);
    expect(sent).toEqual(draft);
    expect(place).toEqual({ collectionId: 'API', folderId: 'Auth' });
    const state = store().states().get('Login');
    expect(state?.phase === 'answered' && state.response.status).toBe(200);
  });

  it('says it is sending until the answer, and sends nothing twice meanwhile', async () => {
    const pending = deferred<SentResponse>();
    http.pendingSend = pending.promise;
    const first = store().send(tab('Login'));
    await Promise.resolve();

    expect(store().isSending('Login')).toBe(true);
    await store().send(tab('Login'));
    expect(http.callsOf('send')).toHaveLength(1);

    pending.resolve(sentResponse({ status: 404, reason: 'Not Found' }));
    await first;
    expect(phase('Login')).toBe('answered');
  });

  it('tells a cancelled send from a failed one, whose code says why', async () => {
    const cancelled = deferred<SentResponse>();
    http.pendingSend = cancelled.promise;
    const sending = store().send(tab('Login'));
    await Promise.resolve();
    await store().cancel('Login');
    expect(http.callsOf('cancel')[0]?.[0]).toMatch(/^Login#/);
    cancelled.reject(new IpcError('send_http_request', { code: 'httpCancelled', params: {}, detail: 'x' }));
    await sending;
    expect(phase('Login')).toBe('cancelled');

    http.failNext = new IpcError('send_http_request', {
      code: 'httpVariable',
      params: { name: 'baseUrl' },
      detail: 'The variable baseUrl has no value',
    });
    await store().send(tab('Login'));
    const state = store().states().get('Login');
    expect(state?.phase === 'failed' && state.notice.ref).toEqual({
      key: 'errors.httpVariable',
      params: { name: 'baseUrl' },
    });
    expect(TestBed.inject(ErrorNotifier).notice()).toBeNull();
  });

  it('follows a draft saved while it waits, and lets Rust drop what a closed tab kept', async () => {
    const pending = deferred<SentResponse>();
    http.pendingSend = pending.promise;
    const sending = store().send(tab('draft-1'));
    await Promise.resolve();
    store().rekey('draft-1', 'request-7');
    pending.resolve(sentResponse());
    await sending;

    expect(phase('draft-1')).toBeUndefined();
    expect(phase('request-7')).toBe('answered');

    store().forget('request-7');
    expect(phase('request-7')).toBeUndefined();
    expect(http.callsOf('forgetResponse')[0]?.[0]).toMatch(/^draft-1#/);
  });

  it('saves the whole body where the user chose, under a name read off its type', async () => {
    http.response = sentResponse({
      headers: [{ enabled: true, key: 'Content-Type', value: 'image/png', description: '' }],
      binary: true,
      cut: true,
    });
    await store().send(tab('Login'));

    await store().saveBody('Login');
    expect(dialog.saveCalls).toHaveLength(1);
    expect(http.callsOf('saveResponse')).toEqual([]);

    dialog.savePath = 'C:/tmp/avatar.png';
    await store().saveBody('Login');
    expect(dialog.saveCalls[1]?.defaultPath).toBe('response.png');
    expect(http.callsOf('saveResponse')[0]?.[1]).toBe('C:/tmp/avatar.png');
    expect(TestBed.inject(StatusNotifier).status()).toEqual({
      key: 'http.response.saved',
      params: { path: 'C:/tmp/avatar.png' },
    });
  });

  it('proposes an extension for the usual types, and .bin failing one', () => {
    const typed = (value: string) =>
      responseFileName(
        sentResponse({ headers: [{ enabled: true, key: 'content-type', value, description: '' }] }),
      );

    expect(typed('application/json; charset=utf-8')).toBe('response.json');
    expect(typed('text/html')).toBe('response.html');
    expect(typed('text/plain')).toBe('response.txt');
    expect(typed('application/octet-stream')).toBe('response.bin');
    expect(responseFileName(sentResponse({ headers: [] }))).toBe('response.bin');
  });
});
