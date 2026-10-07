import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { HttpTabsStore } from '@core/services/http/http-tabs.store';
import { SettingsStore } from '@core/services/settings/settings.store';
import { FakeHttpRepository } from '@testing/fake-http-repository';
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
      parts: { url: '{{baseUrl}}/login', params: [], headers: [], description: '' },
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
    await fixture.whenStable();
    expect(el('[data-testid="http-tab-dirty"]')).toBeNull();
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
