import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { IpcError } from '@core/ipc/ipc.error';
import { EMPTY_PARTS, SentResponse } from '@core/model/http.model';
import { HttpSendStore } from '@core/services/http/http-send.store';
import { FakeHttpRepository, sentResponse } from '@testing/fake-http-repository';
import { provideAppTesting } from '@testing/testing.providers';
import { ResponsePaneComponent } from './response-pane.component';

describe('ResponsePaneComponent', () => {
  let fixture: ComponentFixture<ResponsePaneComponent>;
  let http: FakeHttpRepository;

  beforeEach(async () => {
    TestBed.resetTestingModule();
    http = new FakeHttpRepository();
    TestBed.configureTestingModule({
      imports: [ResponsePaneComponent],
      providers: [provideAppTesting({ httpRepository: http })],
    });
    fixture = TestBed.createComponent(ResponsePaneComponent);
    fixture.componentRef.setInput('tabKey', 'Login');
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  const text = (testId: string) =>
    (fixture.nativeElement as HTMLElement).querySelector(`[data-testid="${testId}"]`)?.textContent?.trim() ??
    null;
  const answer = async (response: SentResponse) => {
    http.response = response;
    await TestBed.inject(HttpSendStore).send({
      key: 'Login',
      place: null,
      draft: { name: 'Login', kind: 'http', method: 'GET', parts: EMPTY_PARTS },
    });
    await fixture.whenStable();
  };

  it('says how to begin before anything was sent', () => {
    expect(text('http-response-idle')).toBe('Envoyez la requête pour lire sa réponse ici.');
  });

  it('shows the status coloured by its class, the time, the size and the body', async () => {
    await answer(sentResponse({ redirects: [{ status: 301, url: 'https://a' }] }));

    expect(text('http-response-status')).toBe('200 OK');
    const status = fixture.nativeElement.querySelector('[data-testid="http-response-status"]') as HTMLElement;
    expect(status.dataset['class']).toBe('2');
    expect(text('http-response-time')).toBe('142 ms');
    expect(text('http-response-size')).toBe('3,2 Ko');
    expect(text('http-response-redirects')).toBe('après 1 redirection');
    expect(text('http-response-body')).toBe('{"data":[]}');
    expect(text('http-response-save')).toBeNull();
  });

  it('says a body was cut, or is not text, and offers it whole as a file', async () => {
    await answer(sentResponse({ size: 2 * 1024 * 1024, cut: true, body: 'aaa' }));
    expect(text('http-response-size')).toBe('2 Mo');
    expect(text('http-response-cut')).toBe('Seul le premier Mo est affiché.');
    expect(text('http-response-save')).toBe('Enregistrer dans un fichier…');

    await answer(
      sentResponse({
        status: 404,
        size: 12,
        headers: [{ enabled: true, key: 'content-type', value: 'image/png', description: '' }],
        body: '',
        binary: true,
        cut: true,
      }),
    );
    expect(text('http-response-size')).toBe('12 o');
    expect(text('http-response-binary')).toBe("Corps binaire (image/png) : il n'est pas affiché.");
    expect(text('http-response-body')).toBeNull();
  });

  it('says why nothing came back', async () => {
    http.failNext = new IpcError('send_http_request', { code: 'httpRefused', params: {}, detail: 'refused' });
    await answer(sentResponse());

    expect(text('http-response-failure')).toBe("La connexion a été refusée : rien n'écoute à cette adresse.");
  });
});
