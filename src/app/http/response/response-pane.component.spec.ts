import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { IpcError } from '@core/ipc/ipc.error';
import { EMPTY_PARTS, SentResponse } from '@core/model/http.model';
import { HttpSendStore } from '@core/services/http/http-send.store';
import { FakeClipboard } from '@testing/fake-clipboard';
import { FakeHttpRepository, sentResponse } from '@testing/fake-http-repository';
import { provideAppTesting } from '@testing/testing.providers';
import { ResponsePaneComponent } from './response-pane.component';

describe('ResponsePaneComponent', () => {
  let fixture: ComponentFixture<ResponsePaneComponent>;
  let http: FakeHttpRepository;
  let clipboard: FakeClipboard;

  beforeEach(async () => {
    TestBed.resetTestingModule();
    http = new FakeHttpRepository();
    clipboard = new FakeClipboard();
    TestBed.configureTestingModule({
      imports: [ResponsePaneComponent],
      providers: [provideAppTesting({ httpRepository: http, clipboard })],
    });
    fixture = TestBed.createComponent(ResponsePaneComponent);
    fixture.componentRef.setInput('tabKey', 'Login');
    fixture.componentRef.setInput('requestName', 'Lister les factures');
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  const el = (testId: string) =>
    (fixture.nativeElement as HTMLElement).querySelector<HTMLElement>(`[data-testid="${testId}"]`);
  const text = (testId: string) => el(testId)?.textContent?.trim() ?? null;
  const all = (testId: string) => [
    ...(fixture.nativeElement as HTMLElement).querySelectorAll<HTMLElement>(`[data-testid="${testId}"]`),
  ];
  const section = async (id: string) => {
    (fixture.nativeElement as HTMLElement)
      .querySelector<HTMLButtonElement>(`[data-testid="http-response-section"][data-section="${id}"]`)!
      .click();
    await fixture.whenStable();
  };
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

  it('shows the status coloured by its class, the time, the size, and counts the headers and cookies', async () => {
    await answer(
      sentResponse({
        redirects: [{ status: 301, url: 'https://a' }],
        cookies: [
          {
            name: 'session',
            value: 'abc',
            domain: null,
            path: '/',
            expires: null,
            httpOnly: true,
            secure: false,
            sameSite: 'Lax',
          },
        ],
      }),
    );

    expect(text('http-response-status')).toBe('200 OK');
    expect(el('http-response-status')!.dataset['class']).toBe('2');
    expect(text('http-response-time')).toBe('142 ms');
    expect(text('http-response-size')).toBe('3,2 Ko');
    expect(all('http-response-count').map((count) => count.textContent?.trim())).toEqual(['1', '1']);
    expect(text('http-response-redirects')).toBe('après 1 redirection');
    expect(el('http-response-pretty')).not.toBeNull();
    expect(text('http-response-save')).toBeNull();

    await section('cookies');
    expect(text('http-response-cookie')).toContain('session');
    expect(text('http-response-cookie')).toContain('SameSite=Lax');
    await section('headers');
    expect(text('http-response-headers')).toContain('application/json');
    await section('timeline');
    expect(text('http-response-sent')).toBe('GET https://api.exemple.fr/users\nAccept: application/json');
    expect(text('http-response-received')).toBe('200 OK\ncontent-type: application/json');
  });

  it('offers a cut body whole as a file, and keeps a binary one out of the clipboard and the notes', async () => {
    await answer(sentResponse({ size: 2 * 1024 * 1024, cut: true, body: 'aaa', pretty: null }));
    expect(text('http-response-size')).toBe('2 Mo');
    expect(text('http-response-save')).toBe('Enregistrer dans un fichier…');

    await answer(
      sentResponse({
        status: 404,
        size: 12,
        headers: [{ enabled: true, key: 'content-type', value: 'application/zip', description: '' }],
        body: '',
        binary: true,
        cut: true,
        pretty: null,
      }),
    );
    expect(el('http-response-status')!.dataset['class']).toBe('4');
    expect(text('http-response-size')).toBe('12 o');
    expect((el('http-response-copy') as HTMLButtonElement).disabled).toBe(true);
    expect((el('http-response-save-note') as HTMLButtonElement).disabled).toBe(true);
  });

  it('copies the body as it came, and saves it laid out as a snippet in its language', async () => {
    await answer(sentResponse());

    el('http-response-copy')!.click();
    await vi.waitFor(() => expect(clipboard.content).toBe('{"data":[]}'));

    el('http-response-save-note')!.click();
    await fixture.whenStable();
    expect((el('save-as-note-title') as HTMLInputElement).value).toBe('Réponse de Lister les factures');
    expect(text('save-as-note-what')).toBe('Un snippet JSON, avec le corps de la réponse mis en forme.');
  });

  it('says why nothing came back', async () => {
    http.failNext = new IpcError('send_http_request', { code: 'httpRefused', params: {}, detail: 'refused' });
    await answer(sentResponse());

    expect(text('http-response-failure')).toBe("La connexion a été refusée : rien n'écoute à cette adresse.");
  });
});
