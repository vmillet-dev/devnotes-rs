import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { EMPTY_PARTS } from '@core/model/http.model';
import { HttpCollectionsStore } from '@core/services/http/http-collections.store';
import { HttpHistoryStore } from '@core/services/http/http-history.store';
import { SettingsStore } from '@core/services/settings/settings.store';
import { HttpTabsStore } from '@core/services/http/http-tabs.store';
import { ClockService } from '@core/services/time/clock.service';
import { FakeHttpRepository } from '@testing/fake-http-repository';
import { historyEntry, historyItem } from '@testing/http-history.fixture';
import { collection, request } from '@testing/http-tree.fixture';
import { provideAppTesting } from '@testing/testing.providers';
import { HistoryPageComponent } from './history-page.component';

describe('HistoryPageComponent', () => {
  let fixture: ComponentFixture<HistoryPageComponent>;
  let http: FakeHttpRepository;

  beforeEach(async () => {
    TestBed.resetTestingModule();
    http = new FakeHttpRepository();
    const failed = historyItem('Payer', {
      method: 'POST',
      status: null,
      summary: { name: 'Payer', url: 'https://x', millis: null, size: null, failure: 'httpRefused' },
    });
    const slow = historyItem('Lent', {
      status: null,
      summary: {
        name: 'Lent',
        url: 'https://x',
        millis: null,
        size: null,
        failure: 'httpTimeout',
        failureParams: { amount: '2', unit: 's', source: 'folder', name: 'Auth' },
      },
    });
    http.historyAnswer = [
      {
        day: '2026-10-07',
        items: [
          historyItem('Login', {
            summary: {
              name: 'Login',
              url: 'https://api.exemple.fr/Login',
              millis: 142,
              size: 3277,
              failure: null,
              environment: { name: 'Staging', colour: 'amber' },
            },
          }),
          failed,
          slow,
        ],
      },
      { day: '2026-10-06', items: [historyItem('Health', { status: 404 })] },
      { day: '2026-10-01', items: [historyItem('Old')] },
    ];
    http.entries.set('Login', historyEntry(historyItem('Login')));
    http.entries.set('Payer', historyEntry(failed, false));
    http.entries.set('Lent', historyEntry(slow, false));
    TestBed.configureTestingModule({
      imports: [HistoryPageComponent],
      providers: [provideAppTesting({ httpRepository: http })],
    });
    vi.spyOn(TestBed.inject(ClockService), 'now').mockReturnValue(new Date(2026, 9, 7, 15, 0));
    TestBed.inject(HttpHistoryStore).open();
    fixture = TestBed.createComponent(HistoryPageComponent);
    fixture.autoDetectChanges();
    await vi.waitFor(() => expect(all('http-history-item')).toHaveLength(5));
  });

  const el = (testId: string) =>
    (fixture.nativeElement as HTMLElement).querySelector<HTMLElement>(`[data-testid="${testId}"]`);
  const all = (testId: string) => [
    ...(fixture.nativeElement as HTMLElement).querySelectorAll<HTMLElement>(`[data-testid="${testId}"]`),
  ];
  const item = (id: string) =>
    (fixture.nativeElement as HTMLElement).querySelector<HTMLButtonElement>(
      `[data-testid="http-history-item"][data-id="${id}"]`,
    )!;

  it('lists the sends by day, today and yesterday by name, each with its status', () => {
    const days = all('http-history-day').map((day) => day.textContent?.trim());
    expect(days.slice(0, 2)).toEqual(['Aujourd’hui', 'Hier']);
    expect(days[2]).toMatch(/1 octobre/);
    expect(
      all('http-history-status').map((status) => [status.textContent?.trim(), status.dataset['class']]),
    ).toEqual([
      ['200', '2'],
      ['échec', 'failed'],
      ['échec', 'failed'],
      ['404', '4'],
      ['200', '2'],
    ]);
  });

  it('says which environment a send left with, and none for one sent without', () => {
    expect(all('http-history-environment').map((environment) => environment.textContent?.trim())).toEqual([
      'Staging',
    ]);
    expect(item('Login').querySelector('[data-testid="http-history-environment"]')).not.toBeNull();
  });

  it('reads an entry like a response, or says why there was none', async () => {
    item('Login').click();
    await vi.waitFor(() => expect(el('http-response-status')?.textContent).toContain('200 OK'));
    expect(el('http-history-url')?.textContent).toBe('https://api.exemple.fr/users');
    expect(el('http-response-save')).toBeNull();

    item('Payer').click();
    await vi.waitFor(() =>
      expect(el('http-history-failure')?.textContent?.trim()).toBe(
        'La connexion a été refusée : rien n’écoute à cette adresse.',
      ),
    );
  });

  /** The values are kept with the code: the sentence is the one the send showed. */
  it('says a timeout with the delay that ran out and who had set it', async () => {
    item('Lent').click();

    await vi.waitFor(() =>
      expect(el('http-history-failure')?.textContent?.trim()).toBe(
        'Pas de réponse au bout de 2 s (réglage hérité du dossier « Auth »).',
      ),
    );
  });

  /**
   * Every row read `https://httpbin.org/hea…`: cut at its end, in a list that could not be
   * widened, fourteen sends to one host looked alike.
   */
  it('writes each URL in two parts, the whole one a hover away, in a list that can be widened', async () => {
    const row = item('Login');
    expect([...row.querySelectorAll('.url-head, .url-tail')].map((part) => part.textContent)).toEqual([
      'api.exemple.fr',
      '/Login',
    ]);
    expect(row.title).toBe('https://api.exemple.fr/Login');

    const edge = el('http-history-edge')!;
    expect(edge.getAttribute('aria-label')).toBe('Largeur de la liste des envois');
    expect(edge.getAttribute('aria-valuenow')).toBe('380');
    edge.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true }));
    await fixture.whenStable();
    expect(TestBed.inject(SettingsStore).historyListWidth()).toBe(396);
    expect(el('http-history-edge')!.getAttribute('aria-valuenow')).toBe('396');
  });

  it('deletes a row from its button or with Suppr, and puts it back from the bar that follows', async () => {
    const rows = () => all('http-history-item').map((row) => row.dataset['id']);
    const row = (id: string) =>
      (fixture.nativeElement as HTMLElement).querySelector<HTMLElement>(
        `[data-testid="http-history-row"][data-id="${id}"]`,
      )!;

    row('Login').querySelector<HTMLButtonElement>('[data-testid="http-history-delete"]')!.click();
    await vi.waitFor(() => expect(rows()).toEqual(['Payer', 'Lent', 'Health', 'Old']));
    expect(el('http-history-undo')?.textContent).toContain('Envoi « Login » supprimé de l’historique.');

    el('http-history-undo-restore')!.click();
    await vi.waitFor(() => expect(http.callsOf('restoreHistoryEntry')).toHaveLength(1));
    await vi.waitFor(() => expect(el('http-history-undo')).toBeNull());

    // The key on the row that holds the focus; with a modifier it is not this one's.
    const payer = item('Payer');
    payer.dispatchEvent(new KeyboardEvent('keydown', { key: 'Delete', ctrlKey: true, bubbles: true }));
    expect(http.callsOf('deleteHistoryEntry')).toEqual([['Login']]);
    payer.dispatchEvent(new KeyboardEvent('keydown', { key: 'Delete', bubbles: true, cancelable: true }));
    await vi.waitFor(() => expect(http.callsOf('deleteHistoryEntry')).toEqual([['Login'], ['Payer']]));

    await vi.waitFor(() => expect(el('http-history-undo-dismiss')).not.toBeNull());
    el('http-history-undo-dismiss')!.click();
    await vi.waitFor(() => expect(el('http-history-undo')).toBeNull());
    expect(http.callsOf('restoreHistoryEntry')).toHaveLength(1);
  });

  it('says where « Rouvrir » puts a send back, and asks before replacing changes', async () => {
    // `Login` came from a request the tree still holds; `Payer` from a draft.
    http.tree$ = { collections: [collection('API', [request('Connexion', 'API')])] };
    http.seed('Connexion', { parts: { ...EMPTY_PARTS, url: 'https://api.exemple.fr/now' } });
    await TestBed.inject(HttpCollectionsStore).load();
    const login = historyEntry(historyItem('Login', { requestId: 'Connexion' }));
    http.entries.set('Login', login);
    http.historyAnswer = [{ day: '2026-10-07', items: [login.item, historyItem('Payer')] }];
    http.ontoAnswer = {
      name: 'Connexion',
      kind: 'http',
      method: 'GET',
      document: { url: 'https://x/then', description: '' },
    };
    const reopen = () => el('http-history-reopen')!;

    item('Payer').click();
    await vi.waitFor(() => expect(reopen().textContent?.trim()).toBe('Rouvrir en brouillon'));
    expect(reopen().dataset['into']).toBe('draft');

    item('Login').click();
    await vi.waitFor(() => expect(reopen().textContent?.trim()).toBe('Rouvrir dans Connexion'));
    expect(reopen().dataset['into']).toBe('request');

    // Its tab is open with changes of its own: asked first, and « Annuler » touches nothing.
    const tabs = TestBed.inject(HttpTabsStore);
    await tabs.open('Connexion');
    tabs.editParts('Connexion', { description: 'en cours' });
    reopen().click();
    await vi.waitFor(() =>
      expect(el('http-history-replace')?.textContent?.trim()).toBe(
        '« Connexion » a des modifications : les remplacer par cet envoi ?',
      ),
    );
    el('http-history-replace-cancel')!.click();
    await fixture.whenStable();
    expect(el('http-history-replace')).toBeNull();
    expect(tabs.active()!.draft.parts.description).toBe('en cours');

    reopen().click();
    await vi.waitFor(() => expect(el('http-history-replace-confirm')).not.toBeNull());
    el('http-history-replace-confirm')!.click();
    await vi.waitFor(() => expect(tabs.active()!.draft.parts.url).toBe('https://x/then'));
    expect(TestBed.inject(HttpHistoryStore).isOpen()).toBe(false);
  });

  it('says how many entries it empties before it does', async () => {
    el('http-history-clear')!.click();
    await vi.waitFor(() =>
      expect(el('http-history-clear-count')?.textContent?.trim()).toBe('Effacer les 5 entrées ?'),
    );
    expect(http.callsOf('clearHistory')).toEqual([]);

    el('http-history-clear-confirm')!.click();
    await vi.waitFor(() => expect(el('http-history-empty')).not.toBeNull());
    expect(http.callsOf('clearHistory')).toHaveLength(1);
  });
});
