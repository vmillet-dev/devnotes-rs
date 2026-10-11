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
      requestId: null,
      draft: { name: 'Login', kind: 'http', method: 'GET', parts: EMPTY_PARTS },
    });
    await fixture.whenStable();
  };

  /**
   * A still sentence made a slow server and a frozen window look the same. The count starts at
   * the store's instant: a pane drawn later, on coming back to the tab, does not restart it.
   */
  it('turns and counts the time since the send while it waits, then stops', async () => {
    vi.useFakeTimers({ toFake: ['Date', 'setInterval', 'clearInterval'] });
    vi.setSystemTime(new Date(2026, 9, 11, 10, 0, 0));
    try {
      let answered!: (response: SentResponse) => void;
      http.pendingSend = new Promise((resolve) => (answered = resolve));
      const sent = TestBed.inject(HttpSendStore).send({
        key: 'Login',
        place: null,
        requestId: null,
        draft: { name: 'Login', kind: 'http', method: 'GET', parts: EMPTY_PARTS },
      });
      await fixture.whenStable();

      expect(el('http-response-sending')?.querySelector('app-spinner')).not.toBeNull();
      expect(text('http-response-elapsed')).toBe('0,0 s');
      expect(el('http-response-elapsed')?.getAttribute('aria-hidden')).toBe('true');

      vi.advanceTimersByTime(2_400);
      await fixture.whenStable();
      expect(text('http-response-elapsed')).toBe('2,4 s');

      const back = TestBed.createComponent(ResponsePaneComponent);
      back.componentRef.setInput('tabKey', 'Login');
      back.componentRef.setInput('requestName', 'Lister les factures');
      back.autoDetectChanges();
      await back.whenStable();
      expect(
        (back.nativeElement as HTMLElement).querySelector('[data-testid="http-response-elapsed"]')
          ?.textContent,
      ).toBe('2,4 s');

      answered(sentResponse());
      await sent;
      await fixture.whenStable();
      await back.whenStable();
      expect(el('http-response-sending')).toBeNull();
      expect(vi.getTimerCount()).toBe(0);
    } finally {
      vi.useRealTimers();
    }
  });

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
    expect(all('http-response-redirect-cookie')).toEqual([]);

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

  /**
   * A login sets its cookie on the redirect: the tab read « La réponse ne pose aucun cookie »,
   * the last answer's headers being all that was looked at.
   */
  it('lists the cookies set along the chain with the hop that set each, and shows them on its redirect', async () => {
    const cookie = (name: string, from: string | null) => ({
      name,
      value: 'v',
      domain: null,
      path: '/',
      expires: null,
      httpOnly: false,
      secure: false,
      sameSite: null,
      from,
    });
    const login = 'https://api.exemple.fr/login';
    await answer(
      sentResponse({
        redirects: [
          { status: 302, from: login, url: 'https://api.exemple.fr/me', cookies: [cookie('session', login)] },
          { status: 301, from: 'https://api.exemple.fr/me', url: 'https://api.exemple.fr/moi', cookies: [] },
        ],
        cookies: [cookie('session', login), cookie('seen', null)],
      }),
    );

    await section('cookies');
    const rows = all('http-response-cookie');
    expect(rows.map((row) => row.dataset['setBy'])).toEqual(['redirect', 'answer']);
    expect(rows.map((row) => row.querySelector('.set-by')?.textContent?.trim())).toEqual([
      login,
      'la réponse',
    ]);

    await section('timeline');
    expect(
      all('http-response-redirect').map((hop) =>
        [...hop.querySelectorAll('.code, .url, .set')].map((piece) => piece.textContent?.trim()),
      ),
    ).toEqual([
      ['302', 'https://api.exemple.fr/me', 'Set-Cookie: session=v'],
      ['301', 'https://api.exemple.fr/moi'],
    ]);
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

  it('reads a GraphQL answer as its data, its errors in a tab of their own', async () => {
    await answer(
      sentResponse({
        body: '{"data":{"me":null},"errors":[{"message":"Unauthorised"}]}',
        graphql: {
          data: '{\n  "me": null\n}',
          errors: [{ message: 'Unauthorised', path: 'me', location: '1:3' }],
        },
      }),
    );

    expect(el('http-response-pretty')?.textContent).toContain('"me": null');
    expect(el('http-response-pretty')?.textContent).not.toContain('errors');
    // With its data it opens on the body: the errors' tab is marked, and the 200 is not alone.
    expect(text('http-response-status')).toBe('200 OK');
    expect(text('http-response-graphql-errors')).toBe('1 erreur');
    const marked = all('http-response-count').find((count) => count.classList.contains('failing'));
    expect(marked?.textContent?.trim()).toBe('1');
    await section('errors');
    expect(
      all('http-response-error').map((error) => [...error.children].map((part) => part.textContent?.trim())),
    ).toEqual([['Unauthorised', 'me', 'ligne:colonne 1:3']]);
  });

  /**
   * A query missing a brace: the server answers 400 with its errors, the pane opened on
   * « Corps », which read `null`, and the errors waited in a tab nothing pointed to.
   */
  it('opens on the errors of a GraphQL answer that carries no data, and on the body of the next one', async () => {
    const errors = [
      { message: 'Syntax Error: Expected Name, found <EOF>.', path: null, location: '4:1' },
      { message: 'Unknown field', path: null, location: null },
    ];
    await answer(
      sentResponse({
        status: 400,
        reason: 'Bad Request',
        body: '{"errors":[]}',
        graphql: { data: null, errors },
      }),
    );

    expect(el('http-response-errors')).not.toBeNull();
    expect(all('http-response-error')).toHaveLength(2);
    expect(text('http-response-graphql-errors')).toBe('2 erreurs');
    expect(
      all('http-response-section').find((tab) => tab.getAttribute('aria-selected') === 'true')?.dataset[
        'section'
      ],
    ).toBe('errors');
    // « Brut » still shows the answer as it came.
    await section('body');
    expect(el('http-response-errors')).toBeNull();

    // An answer that reads opens on its body, the errors of the last one behind it.
    await section('errors');
    await answer(sentResponse({ graphql: { data: '{\n  "me": 1\n}', errors: [] } }));
    expect(el('http-response-errors')).toBeNull();
    expect(el('http-response-pretty')?.textContent).toContain('"me": 1');
    expect(text('http-response-graphql-errors')).toBeNull();

    // And one that is no GraphQL answer keeps the tab that was being read.
    await section('headers');
    await answer(sentResponse());
    expect(el('http-response-headers')).not.toBeNull();
  });

  it('says why nothing came back', async () => {
    http.failNext = new IpcError('send_http_request', { code: 'httpRefused', params: {}, detail: 'refused' });
    await answer(sentResponse());

    expect(text('http-response-failure')).toBe("La connexion a été refusée : rien n'écoute à cette adresse.");
  });
});
