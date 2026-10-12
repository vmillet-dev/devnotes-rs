import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { HttpTabsStore } from '@core/services/http/http-tabs.store';
import { SettingsStore } from '@core/services/settings/settings.store';
import { DEFAULT_TRANSPORT, EMPTY_PARTS, ImpliedHeader, SentResponse } from '@core/model/http.model';
import { FakeHttpRepository, sentResponse } from '@testing/fake-http-repository';
import { sampleTree } from '@testing/http-tree.fixture';
import { provideAppTesting } from '@testing/testing.providers';
import { HttpPageComponent } from './http-page.component';

describe('HttpPageComponent', () => {
  let fixture: ComponentFixture<HttpPageComponent>;
  let http: FakeHttpRepository;

  beforeEach(async () => {
    TestBed.resetTestingModule();
    http = new FakeHttpRepository();
    http.tree$ = sampleTree();
    // Where the tree holds it: a tab follows the tree for its request's name and place.
    http.seed(
      'Login',
      {
        method: 'POST',
        parts: { ...EMPTY_PARTS, url: '{{baseUrl}}/login' },
      },
      'API',
      'Auth',
    );
    TestBed.configureTestingModule({
      imports: [HttpPageComponent],
      providers: [provideAppTesting({ httpRepository: http })],
    });
    fixture = TestBed.createComponent(HttpPageComponent);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  const el = <E extends HTMLElement = HTMLElement>(selector: string): E =>
    fixture.nativeElement.querySelector(selector);
  const all = (selector: string): HTMLElement[] => [...fixture.nativeElement.querySelectorAll(selector)];
  const tabs = () => TestBed.inject(HttpTabsStore);
  const key = (event: Partial<KeyboardEventInit>) =>
    document.dispatchEvent(new KeyboardEvent('keydown', { ctrlKey: true, cancelable: true, ...event }));
  /** A header Rust says leaves unasked; `from` only for one handed down. */
  const implied = (
    name: string,
    value: string,
    source: ImpliedHeader['source'],
    from: ImpliedHeader['from'] = null,
  ): ImpliedHeader => ({ name, value, source, from });
  const impliedRows = () =>
    all('[data-testid="http-implied-header"]').map((row) =>
      [...row.children].map((cell) => cell.textContent?.trim()).join(' · '),
    );
  const typeInto = async (selector: string, text: string) => {
    const field = el<HTMLInputElement>(selector);
    field.value = text;
    field.dispatchEvent(new Event('input'));
    await fixture.whenStable();
  };

  it('reads the collections when it opens, and says how to begin while no tab is open', async () => {
    await vi.waitFor(() => expect(http.callsOf('tree').length).toBeGreaterThan(0));

    expect(el('[data-testid="http-rail"]')).not.toBeNull();
    expect(el('[data-testid="http-workspace-empty"]')).not.toBeNull();
  });

  it('shows the open request in its tab: its method, its name, its URL with the variable lit', async () => {
    await tabs().open('Login');
    await fixture.whenStable();

    expect(all('[data-testid="http-tab"]').map((tab) => tab.dataset['key'])).toEqual(['Login']);
    expect(el<HTMLInputElement>('[data-testid="http-request-name"]').value).toBe('Login');
    expect(el<HTMLInputElement>('[data-testid="http-url"]').value).toBe('{{baseUrl}}/login');
    expect(all('[data-testid="http-url-variable"]').map((piece) => piece.textContent)).toEqual([
      '{{baseUrl}}',
    ]);
    expect(el<HTMLButtonElement>('[data-testid="http-save"]').disabled).toBe(true);
  });

  it('edits the URL and the params in step, counts them, and saves on Ctrl+S', async () => {
    await tabs().open('Login');
    await fixture.whenStable();
    http.synced = {
      url: '{{baseUrl}}/login?page=1',
      params: [{ enabled: true, key: 'page', value: '1', description: '' }],
    };

    await typeInto('[data-testid="http-url"]', '{{baseUrl}}/login?page=1');
    await vi.waitFor(() => expect(el('[data-testid="http-section-count"]')?.textContent).toBe('1'));
    expect(el('[data-testid="http-tab-dirty"]')).not.toBeNull();
    expect(all('[data-testid="http-kv-key"]').map((field) => (field as HTMLInputElement).value)).toEqual([
      'page',
      '',
    ]);

    key({ key: 's' });
    await vi.waitFor(() => expect(http.callsOf('saveRequest')).toHaveLength(1));
    await vi.waitFor(() => expect(el('[data-testid="http-tab-dirty"]')).toBeNull());
  });

  it('switches to the headers, proposing the names a request sends', async () => {
    await tabs().open('Login');
    await fixture.whenStable();

    el<HTMLButtonElement>('[data-testid="http-section"][data-section="headers"]').click();
    await fixture.whenStable();
    await typeInto('[data-testid="http-headers-table"] [data-testid="http-kv-key"]', 'Accept');

    expect(tabs().active()!.draft.parts.headers).toEqual([
      { enabled: true, key: 'Accept', value: '', description: '' },
    ]);
    const names = all('datalist option').map((option) => (option as HTMLOptionElement).value);
    expect(names).toContain('Authorization');
    expect(names).not.toContain('Set-Cookie');
  });

  it('places a new request the first time it is saved, then reads the tree again', async () => {
    el<HTMLButtonElement>('[data-testid="http-tab-new"]').click();
    await fixture.whenStable();
    el<HTMLButtonElement>('[data-testid="http-save"]').click();
    await fixture.whenStable();

    expect(el('[data-testid="http-save-dialog"]')).not.toBeNull();
    await typeInto('[data-testid="http-save-name"]', 'Santé');
    el<HTMLButtonElement>('[data-testid="http-save-submit"]').click();

    await vi.waitFor(() => expect(http.callsOf('createRequest')).toHaveLength(1));
    expect(http.callsOf('createRequest')[0]?.[0]).toMatchObject({
      collectionId: 'API',
      folderId: null,
      name: 'Santé',
    });
    await vi.waitFor(() => expect(tabs().activeRequestId()).toMatch(/^request-/));
    expect(el('[data-testid="http-save-dialog"]')).toBeNull();
  });

  it('asks before closing a modified tab: cancel, discard, or save then close', async () => {
    await tabs().open('Login');
    tabs().edit('Login', { name: 'Se connecter' });
    await fixture.whenStable();

    const close = () => el<HTMLButtonElement>('[data-testid="http-tab-close"]').click();
    close();
    await fixture.whenStable();
    el<HTMLButtonElement>('[data-testid="http-close-cancel"]').click();
    await fixture.whenStable();
    expect(tabs().tabs()).toHaveLength(1);

    close();
    await fixture.whenStable();
    el<HTMLButtonElement>('[data-testid="http-close-save"]').click();
    await vi.waitFor(() => expect(tabs().tabs()).toHaveLength(0));
    expect(http.callsOf('saveRequest')[0]?.[1]).toMatchObject({ name: 'Se connecter' });

    tabs().newRequest();
    await fixture.whenStable();
    close();
    await fixture.whenStable();
    el<HTMLButtonElement>('[data-testid="http-close-discard"]').click();
    await fixture.whenStable();
    expect(tabs().tabs()).toEqual([]);
  });

  it('shows the auth a request inherits, its body, and the headers both hand it', async () => {
    http.inheritedAnswer = {
      auth: { kind: 'bearer', token: '{{accessToken}}' },
      authFrom: { kind: 'collection', id: 'API', name: 'API Paiements' },
      headers: [
        {
          header: { enabled: true, key: 'Accept', value: 'application/json', description: '' },
          from: { kind: 'collection', id: 'API', name: 'API Paiements' },
        },
      ],
      transport: DEFAULT_TRANSPORT,
    };
    http.bodyAnswer = { contentType: 'application/json', problem: null, unusual: false };
    http.impliedAnswer = [
      implied('Content-Type', 'application/json', 'body'),
      implied('Accept', 'application/json', 'above', {
        kind: 'collection',
        id: 'API',
        name: 'API Paiements',
      }),
      implied('X-Trace', '1', 'above', { kind: 'folder', id: 'Auth', name: 'Auth' }),
      implied('User-Agent', 'DevNotes/0.10.1', 'default'),
      implied('Host', 'api.exemple.fr', 'url'),
    ];
    await tabs().open('Login');
    await fixture.whenStable();
    expect(el('[data-testid="http-auth-inherits"]')?.textContent?.trim()).toBe('héritée');

    el<HTMLButtonElement>('[data-testid="http-section"][data-section="auth"]').click();
    await vi.waitFor(() =>
      expect(el('[data-testid="http-auth-inherited"]')?.textContent).toContain(
        'héritée de la collection API Paiements',
      ),
    );
    expect(http.callsOf('inherited')[0]).toEqual(['API', 'Auth']);

    el<HTMLButtonElement>('[data-testid="http-section"][data-section="body"]').click();
    await fixture.whenStable();
    el<HTMLButtonElement>('[data-testid="segmented-http-body"] [data-segment-id="json"]').click();
    await vi.waitFor(() => expect(tabs().active()!.draft.parts.body).toEqual({ kind: 'json', text: '' }));

    el<HTMLButtonElement>('[data-testid="http-section"][data-section="headers"]').click();
    await vi.waitFor(() => expect(all('[data-testid="http-implied-header"]')).toHaveLength(5));
    expect(impliedRows()).toEqual([
      "Content-Type · application/json · d'après le corps",
      'Accept · application/json · héritée de la collection API Paiements',
      'X-Trace · 1 · héritée du dossier Auth',
      'User-Agent · DevNotes/0.10.1 · par défaut',
      "Host · api.exemple.fr · d'après l'URL",
    ]);
    expect(all('[data-testid="http-implied-header"]').map((row) => row.dataset['source'])).toEqual([
      'body',
      'above',
      'above',
      'default',
      'url',
    ]);
    // Asked of Rust with the request as it stands and where it sits: what a send would read.
    expect(http.callsOf('impliedHeaders').at(-1)).toEqual([
      tabs().active()!.draft,
      { collectionId: 'API', folderId: 'Auth' },
    ]);
  });

  /**
   * The list was decided here, from the body alone: a request left with no `User-Agent`, and
   * nothing said so. Rust composes the request, so the list is read from it — again each time
   * what it depends on changes, and only then.
   */
  it('asks Rust what leaves unasked when the URL, a header or the kind changes, not for the rest', async () => {
    http.impliedAnswer = [implied('User-Agent', 'DevNotes/0.10.1', 'default')];
    await tabs().open('Login');
    await fixture.whenStable();
    el<HTMLButtonElement>('[data-testid="http-section"][data-section="headers"]').click();
    await vi.waitFor(() => expect(impliedRows()).toEqual(['User-Agent · DevNotes/0.10.1 · par défaut']));
    const asked = () => http.callsOf('impliedHeaders').length;
    const settled = async () => {
      await fixture.whenStable();
      return asked();
    };
    const before = await settled();

    tabs().editParts('Login', { description: 'Se connecter' });
    tabs().editParts('Login', { body: { kind: 'json', text: '{' } });
    expect(await settled()).toBe(before + 1);
    tabs().editParts('Login', { body: { kind: 'json', text: '{"a":1}' } });
    expect(await settled()).toBe(before + 1);

    // The answer on its way: the list keeps what it showed, the table under it does not jump.
    let answer!: (headers: ImpliedHeader[]) => void;
    http.pendingImplied = new Promise((resolve) => (answer = resolve));
    // Not `typeInto`: nothing is stable while Rust has not answered.
    const name = el<HTMLInputElement>('[data-testid="http-headers-table"] [data-testid="http-kv-key"]');
    name.value = 'User-Agent';
    name.dispatchEvent(new Event('input'));
    await vi.waitFor(() => expect(asked()).toBe(before + 2));
    expect(impliedRows()).toEqual(['User-Agent · DevNotes/0.10.1 · par défaut']);

    answer([implied('Accept', '*/*', 'default')]);
    await vi.waitFor(() => expect(impliedRows()).toEqual(['Accept · */* · par défaut']));

    http.pendingImplied = null;
    http.impliedAnswer = [];
    await typeInto('[data-testid="http-url"]', 'https://api.exemple.fr');
    await vi.waitFor(() => expect(el('[data-testid="http-implied-headers"]')).toBeNull());
    expect(asked()).toBe(before + 3);
  });

  /**
   * A `POST` switched to `GET` kept its JSON body and sent it, and nothing said so — least of
   * all from « Paramètres », the tab in front when the method changes.
   */
  it('marks « Corps » when the method changes over a body, and loses no text on the way to « Aucun »', async () => {
    const mark = () => el('[data-testid="http-body-unusual-mark"]');
    const section = (id: string) =>
      el<HTMLButtonElement>(`[data-testid="http-section"][data-section="${id}"]`).click();
    const method = async (id: string) => {
      el<HTMLButtonElement>('[data-testid="http-method"]').click();
      await fixture.whenStable();
      el<HTMLButtonElement>(`[data-testid="choice-option"][data-option-id="${id}"]`).click();
    };
    await tabs().open('Login');
    await fixture.whenStable();
    tabs().editParts('Login', { body: { kind: 'json', text: '{"a":1}' } });
    await fixture.whenStable();
    expect(mark()).toBeNull();

    // Rust is asked with the method, and says what the mark and the line show.
    http.bodyAnswer = { contentType: 'application/json', problem: null, unusual: true };
    await method('GET');
    await vi.waitFor(() => expect(mark()?.textContent?.trim()).toBe('sur un GET'));
    expect(http.callsOf('describeBody').at(-1)).toEqual([{ kind: 'json', text: '{"a":1}' }, 'GET']);
    expect(mark()!.getAttribute('title')).toContain('Un GET avec un corps');

    section('body');
    await vi.waitFor(() => expect(el('[data-testid="http-body-unusual"]')).not.toBeNull());
    http.bodyAnswer = { contentType: null, problem: null, unusual: false };
    el<HTMLButtonElement>('[data-testid="http-body-to-none"]').click();
    await vi.waitFor(() => expect(mark()).toBeNull());
    expect(tabs().active()!.draft.parts.body).toEqual({ kind: 'none' });
    expect(el('[data-testid="http-body-unusual"]')).toBeNull();

    http.bodyAnswer = { contentType: 'application/json', problem: null, unusual: true };
    el<HTMLButtonElement>('[data-testid="segmented-http-body"] [data-segment-id="json"]').click();
    await vi.waitFor(() => expect(mark()).not.toBeNull());
    expect(tabs().active()!.draft.parts.body).toEqual({ kind: 'json', text: '{"a":1}' });
    expect(el<HTMLTextAreaElement>('[data-testid="http-body-text"]').value).toBe('{"a":1}');

    http.bodyAnswer = { contentType: 'application/json', problem: null, unusual: false };
    await method('POST');
    await vi.waitFor(() => expect(mark()).toBeNull());
    expect(http.callsOf('describeBody').at(-1)).toEqual([{ kind: 'json', text: '{"a":1}' }, 'POST']);
  });

  it('sets how a request travels in its Réglages, over what its collection hands down', async () => {
    http.inheritedAnswer = { ...http.inheritedAnswer, transport: { ...DEFAULT_TRANSPORT, timeoutMs: 5000 } };
    await tabs().open('Login');
    await fixture.whenStable();
    el<HTMLButtonElement>('[data-testid="http-section"][data-section="settings"]').click();
    await vi.waitFor(() =>
      expect(el('[data-testid="http-transport-from"]')?.textContent).toContain('5000 ms'),
    );

    const timeout = el<HTMLInputElement>('[data-testid="http-transport-timeoutMs"]');
    timeout.value = '2000';
    timeout.dispatchEvent(new Event('change'));
    await fixture.whenStable();

    expect(tabs().active()!.draft.parts.transport).toEqual({ timeoutMs: 2000 });
    expect(
      el('[data-testid="http-section"][data-section="settings"] [data-testid="http-section-count"]')
        ?.textContent,
    ).toBe('1');
  });

  it('closes, when it opens again, the tab of a request the library no longer holds', async () => {
    http.seed('Gone');
    await tabs().open('Gone');
    await tabs().open('Login');
    fixture.destroy();

    fixture = TestBed.createComponent(HttpPageComponent);
    fixture.autoDetectChanges();
    await vi.waitFor(() =>
      expect(
        tabs()
          .tabs()
          .map((tab) => tab.key),
      ).toEqual(['Login']),
    );
  });

  it('sends the open request on Ctrl+Enter, offers to cancel while it waits, then shows the answer', async () => {
    await tabs().open('Login');
    await fixture.whenStable();
    let answer!: (response: SentResponse) => void;
    http.pendingSend = new Promise((resolve) => (answer = resolve));

    key({ key: 'Enter' });
    await fixture.whenStable();
    expect(http.callsOf('send')[0]?.[2]).toEqual({ collectionId: 'API', folderId: 'Auth' });
    expect(el('[data-testid="http-send"]').textContent?.trim()).toBe('Annuler');
    expect(el('[data-testid="http-send"] app-spinner')).not.toBeNull();
    expect(el('[data-testid="http-response-sending"]')).not.toBeNull();
    // Its tab says it too: a request sent then left behind still says it runs.
    expect(el('[data-testid="http-tab"] [data-testid="http-tab-sending"]')).not.toBeNull();

    el<HTMLButtonElement>('[data-testid="http-send"]').click();
    await vi.waitFor(() => expect(http.callsOf('cancel')).toHaveLength(1));
    answer(sentResponse({ status: 201, reason: 'Created' }));
    await vi.waitFor(() =>
      expect(el('[data-testid="http-response-status"]')?.textContent).toContain('201 Created'),
    );
    expect(el('[data-testid="http-send"]').textContent).toContain('Envoyer');
    expect(el('[data-testid="http-send"] app-spinner')).toBeNull();
    expect(el('[data-testid="http-tab-sending"]')).toBeNull();

    el<HTMLButtonElement>('[data-testid="http-tab-close"]').click();
    await fixture.whenStable();
    expect(http.callsOf('forgetResponse')).toHaveLength(1);
  });

  /**
   * GraphQL and WebSocket were the last two entries of the method menu — a kind filed among
   * verbs, and not found; then behind a bare « + » that named nothing. « Nouvelle requête » is
   * a split button, as the notes' is: HTTP in one click, the two others behind its chevron.
   */
  it('begins an HTTP request from « Nouvelle requête », and the kind chosen from its chevron', async () => {
    const begin = async (kind: string) => {
      el<HTMLButtonElement>('[data-testid="http-tab-new-kind"]').click();
      await fixture.whenStable();
      el<HTMLButtonElement>(`[data-testid="http-new-kind"][data-kind="${kind}"]`).click();
      await fixture.whenStable();
    };
    const main = el<HTMLButtonElement>('[data-testid="http-tab-new"]');
    expect(main.textContent?.trim()).toBe('+ Nouvelle requête');

    // One click, and no menu on the way.
    main.click();
    await fixture.whenStable();
    expect(el('[data-testid="http-new-kind-panel"]')).toBeNull();
    expect(tabs().active()?.draft.kind).toBe('http');

    const chevron = el<HTMLButtonElement>('[data-testid="http-tab-new-kind"]');
    expect(chevron.getAttribute('aria-label')).toBe('Choisir le type de requête');
    chevron.click();
    await fixture.whenStable();
    expect(chevron.getAttribute('aria-expanded')).toBe('true');
    expect(
      all('[data-testid="http-new-kind"]').map((entry) => [
        entry.querySelector('.glyph')?.textContent?.trim(),
        entry.querySelector('.name')?.textContent?.trim(),
      ]),
    ).toEqual([
      ['GET', 'Requête HTTP'],
      ['GQL', 'Requête GraphQL'],
      ['WS', 'WebSocket'],
    ]);
    // Each says what it is for, as a kind of note does.
    expect(el('[data-testid="http-new-kind"][data-kind="websocket"] .line').textContent?.trim()).toBe(
      'Une connexion ouverte : des messages dans les deux sens.',
    );
    await vi.waitFor(() => expect(document.activeElement?.getAttribute('data-kind')).toBe('http'));
    el<HTMLButtonElement>('[data-testid="http-new-kind"][data-kind="http"]').click();
    await fixture.whenStable();
    expect(el('[data-testid="http-new-kind-panel"]')).toBeNull();

    await begin('graphql');
    await begin('websocket');
    expect(
      tabs()
        .tabs()
        .map((tab) => tab.draft.kind),
    ).toEqual(['http', 'http', 'graphql', 'websocket']);
  });

  it('says what the area can do while no tab is open: the three kinds, a button each', async () => {
    expect(all('[data-testid="http-workspace-new"]').map((button) => button.dataset['kind'])).toEqual([
      'http',
      'graphql',
      'websocket',
    ]);

    el<HTMLButtonElement>('[data-testid="http-workspace-new"][data-kind="graphql"]').click();
    await fixture.whenStable();

    expect(tabs().active()?.draft.kind).toBe('graphql');
    expect(el('[data-testid="http-workspace-empty"]')).toBeNull();
    expect(el('[data-testid="http-graphql"]')).not.toBeNull();
  });

  it('shows a GraphQL request with its kind where a method would be chosen, its query in place of the body', async () => {
    http.seed(
      'Produits',
      { kind: 'graphql', parts: { ...EMPTY_PARTS, url: 'https://x/graphql' } },
      'Catalogue',
    );
    await tabs().open('Produits');
    await fixture.whenStable();

    // Its kind is its own for good: a label, and no menu to turn it into something else.
    expect(el('[data-testid="http-kind-badge"]').textContent?.trim()).toBe('GraphQL');
    expect(el('[data-testid="http-method"]')).toBeNull();
    expect(all('[data-testid="http-section"]').map((section) => section.dataset['section'])).toEqual([
      'query',
      'headers',
      'auth',
      'settings',
    ]);
    expect(el('[data-testid="http-graphql"]')).not.toBeNull();
    // What a GraphQL request implies is Rust's to say: it is asked as one.
    await vi.waitFor(() =>
      expect(http.callsOf('impliedHeaders').at(-1)).toMatchObject([{ kind: 'graphql' }, {}]),
    );
  });

  it('shows a WebSocket with its kind, a socket’s URL as its example, connected from its button', async () => {
    http.seed('Live', { kind: 'websocket' }, 'Flux');
    await tabs().open('Live');
    await fixture.whenStable();

    expect(el('[data-testid="http-kind-badge"]').textContent?.trim()).toBe('WS');
    expect(el('[data-testid="http-method"]')).toBeNull();
    expect(el<HTMLInputElement>('[data-testid="http-url"]').placeholder).toBe('wss://api.exemple.fr/flux');
    expect(all('[data-testid="http-section"]').map((section) => section.dataset['section'])).toEqual([
      'message',
      'headers',
      'auth',
      'protocols',
    ]);
    expect(el('[data-testid="http-send"]')).toBeNull();
    expect(el('[data-testid="http-socket-log"]')).not.toBeNull();

    el<HTMLButtonElement>('[data-testid="http-socket-connect"]').click();
    await vi.waitFor(() => expect(http.callsOf('connectWebsocket')).toHaveLength(1));
    await vi.waitFor(() =>
      expect(el('[data-testid="http-socket-connect"]').textContent?.trim()).toBe('Déconnecter'),
    );
  });

  /** The methods' menu holds methods: what is chosen there never changes what the request is. */
  it('keeps the method menu to methods, on an HTTP request alone', async () => {
    await tabs().open('Login');
    await fixture.whenStable();
    expect(el('[data-testid="http-kind-badge"]')).toBeNull();

    el<HTMLButtonElement>('[data-testid="http-method"]').click();
    await fixture.whenStable();
    expect(all('[data-testid="choice-option"]').map((option) => option.dataset['optionId'])).toEqual([
      'GET',
      'POST',
      'PUT',
      'PATCH',
      'DELETE',
      'HEAD',
      'OPTIONS',
    ]);
    el<HTMLButtonElement>('[data-testid="choice-option"][data-option-id="PUT"]').click();
    await fixture.whenStable();
    expect(tabs().active()!.draft).toMatchObject({ kind: 'http', method: 'PUT' });
  });

  /** Closing the tab of an open socket cut it without a word. */
  it('asks before closing a tab whose WebSocket is connected, in one dialog with what is not saved', async () => {
    const close = () => el<HTMLButtonElement>('[data-testid="http-tab-close"]').click();
    // Where the tree holds it: a tab whose request is in no tree closes.
    http.seed('Live', { kind: 'websocket', parts: { ...EMPTY_PARTS, url: 'ws://flux.exemple.fr' } }, 'Flux');
    await tabs().open('Live');
    await fixture.whenStable();
    el<HTMLButtonElement>('[data-testid="http-socket-connect"]').click();
    await vi.waitFor(() => expect(http.callsOf('connectWebsocket')).toHaveLength(1));

    // Connected and nothing to save: one sentence, « Fermer » or « Annuler ».
    close();
    await fixture.whenStable();
    expect(el('[data-testid="http-close-connected"]').textContent?.trim()).toBe(
      "Le WebSocket est connecté : fermer l'onglet coupe la connexion.",
    );
    expect(el('[data-testid="http-close-modified"]')).toBeNull();
    expect(el('[data-testid="http-close-save"]')).toBeNull();
    expect(el('[data-testid="http-close-discard"]').textContent?.trim()).toBe('Fermer');
    el<HTMLButtonElement>('[data-testid="http-close-cancel"]').click();
    await fixture.whenStable();
    expect(tabs().tabs()).toHaveLength(1);
    expect(http.callsOf('closeWebsocket')).toEqual([]);

    // With changes too: both are said at once, not one dialog after the other.
    tabs().edit('Live', { name: 'Flux temps réel' });
    await fixture.whenStable();
    close();
    await fixture.whenStable();
    expect(el('[data-testid="http-close-connected"]')).not.toBeNull();
    expect(el('[data-testid="http-close-modified"]')).not.toBeNull();
    expect(el('[data-testid="http-close-save"]')).not.toBeNull();
    expect(el('[data-testid="http-close-discard"]').textContent?.trim()).toBe('Fermer sans enregistrer');

    el<HTMLButtonElement>('[data-testid="http-close-discard"]').click();
    await vi.waitFor(() => expect(tabs().tabs()).toEqual([]));
    expect(http.callsOf('closeWebsocket')).toHaveLength(1);
  });

  it('closes an unmodified tab at once', async () => {
    await tabs().open('Login');
    await fixture.whenStable();
    el<HTMLButtonElement>('[data-testid="http-tab-close"]').click();
    await fixture.whenStable();
    expect(tabs().tabs()).toEqual([]);
  });

  /** Hidden, the rail left no control to bring it back: only a key nobody is told about. */
  it('hides its rail with the others, and carries the button that brings it back', async () => {
    const settings = TestBed.inject(SettingsStore);
    const toggle = () => el<HTMLButtonElement>('[data-testid="http-rail-toggle"]');
    expect(toggle().getAttribute('aria-pressed')).toBe('true');

    toggle().click();
    await fixture.whenStable();
    expect(settings.showLibraryRail()).toBe(false);
    expect(el('[data-testid="http-rail"]')).toBeNull();
    expect(toggle().getAttribute('aria-pressed')).toBe('false');

    toggle().click();
    await fixture.whenStable();
    expect(el('[data-testid="http-rail"]')).not.toBeNull();
  });
});
