import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { HttpTabsStore } from '@core/services/http/http-tabs.store';
import { SettingsStore } from '@core/services/settings/settings.store';
import { EMPTY_PARTS, SentResponse } from '@core/model/http.model';
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
    http.seed('Login', {
      method: 'POST',
      parts: { ...EMPTY_PARTS, url: '{{baseUrl}}/login' },
    });
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
    };
    http.bodyAnswer = { contentType: 'application/json', problem: null };
    await tabs().open('Login');
    await fixture.whenStable();
    expect(el('[data-testid="http-auth-inherits"]')?.textContent?.trim()).toBe('héritée');

    el<HTMLButtonElement>('[data-testid="http-section"][data-section="auth"]').click();
    await vi.waitFor(() =>
      expect(el('[data-testid="http-auth-inherited"]')?.textContent).toContain(
        'héritée de la collection API Paiements',
      ),
    );
    expect(http.callsOf('inherited')[0]).toEqual(['API', null]);

    el<HTMLButtonElement>('[data-testid="http-section"][data-section="body"]').click();
    await fixture.whenStable();
    el<HTMLButtonElement>('[data-testid="segmented-http-body"] [data-segment-id="json"]').click();
    await vi.waitFor(() => expect(tabs().active()!.draft.parts.body).toEqual({ kind: 'json', text: '' }));

    el<HTMLButtonElement>('[data-testid="http-section"][data-section="headers"]').click();
    await vi.waitFor(() => expect(all('[data-testid="http-implied-header"]')).toHaveLength(2));
    expect(
      all('[data-testid="http-implied-header"]').map((row) =>
        [...row.children].map((cell) => cell.textContent?.trim()).join(' · '),
      ),
    ).toEqual([
      "Content-Type · application/json · d'après le corps",
      'Accept · application/json · héritée de la collection API Paiements',
    ]);
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
    expect(http.callsOf('send')[0]?.[2]).toEqual({ collectionId: 'API', folderId: null });
    expect(el('[data-testid="http-send"]').textContent?.trim()).toBe('Annuler');
    expect(el('[data-testid="http-response-sending"]')).not.toBeNull();

    el<HTMLButtonElement>('[data-testid="http-send"]').click();
    await vi.waitFor(() => expect(http.callsOf('cancel')).toHaveLength(1));
    answer(sentResponse({ status: 201, reason: 'Created' }));
    await vi.waitFor(() =>
      expect(el('[data-testid="http-response-status"]')?.textContent).toContain('201 Created'),
    );
    expect(el('[data-testid="http-send"]').textContent).toContain('Envoyer');

    el<HTMLButtonElement>('[data-testid="http-tab-close"]').click();
    await fixture.whenStable();
    expect(http.callsOf('forgetResponse')).toHaveLength(1);
  });

  it('turns a request into a GraphQL one from the method menu, its query in place of the body', async () => {
    await tabs().open('Login');
    await fixture.whenStable();

    el<HTMLButtonElement>('[data-testid="http-method"]').click();
    await fixture.whenStable();
    el<HTMLButtonElement>('[data-testid="choice-option"][data-option-id="GRAPHQL"]').click();
    await fixture.whenStable();

    expect(tabs().active()!.draft.kind).toBe('graphql');
    expect(all('[data-testid="http-section"]').map((section) => section.dataset['section'])).toEqual([
      'query',
      'headers',
      'auth',
    ]);
    expect(el('[data-testid="http-graphql"]')).not.toBeNull();
    el<HTMLButtonElement>('[data-testid="http-section"][data-section="headers"]').click();
    await fixture.whenStable();
    expect(el('[data-testid="http-implied-header"]')?.textContent).toContain('application/json');
  });

  it('closes an unmodified tab at once, and hides the rail on Ctrl+B', async () => {
    await tabs().open('Login');
    await fixture.whenStable();
    el<HTMLButtonElement>('[data-testid="http-tab-close"]').click();
    await fixture.whenStable();
    expect(tabs().tabs()).toEqual([]);

    const settings = TestBed.inject(SettingsStore);
    const shown = settings.showLibraryRail();
    key({ key: 'b' });
    await fixture.whenStable();
    expect(settings.showLibraryRail()).toBe(!shown);
  });
});
