import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { HttpHistoryStore } from '@core/services/http/http-history.store';
import { ClockService } from '@core/services/time/clock.service';
import { FakeHttpRepository } from '@testing/fake-http-repository';
import { historyEntry, historyItem } from '@testing/http-history.fixture';
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
    http.historyAnswer = [
      { day: '2026-10-07', items: [historyItem('Login'), failed] },
      { day: '2026-10-06', items: [historyItem('Health', { status: 404 })] },
      { day: '2026-10-01', items: [historyItem('Old')] },
    ];
    http.entries.set('Login', historyEntry(historyItem('Login')));
    http.entries.set('Payer', historyEntry(failed, false));
    TestBed.configureTestingModule({
      imports: [HistoryPageComponent],
      providers: [provideAppTesting({ httpRepository: http })],
    });
    vi.spyOn(TestBed.inject(ClockService), 'now').mockReturnValue(new Date(2026, 9, 7, 15, 0));
    TestBed.inject(HttpHistoryStore).open();
    fixture = TestBed.createComponent(HistoryPageComponent);
    fixture.autoDetectChanges();
    await vi.waitFor(() => expect(all('http-history-item')).toHaveLength(4));
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
    expect(days.slice(0, 2)).toEqual(["Aujourd'hui", 'Hier']);
    expect(days[2]).toMatch(/1 octobre/);
    expect(
      all('http-history-status').map((status) => [status.textContent?.trim(), status.dataset['class']]),
    ).toEqual([
      ['200', '2'],
      ['échec', 'failed'],
      ['404', '4'],
      ['200', '2'],
    ]);
  });

  it('reads an entry like a response, or says why there was none', async () => {
    item('Login').click();
    await vi.waitFor(() => expect(el('http-response-status')?.textContent).toContain('200 OK'));
    expect(el('http-history-url')?.textContent).toBe('https://api.exemple.fr/users');
    expect(el('http-response-save')).toBeNull();

    item('Payer').click();
    await vi.waitFor(() =>
      expect(el('http-history-failure')?.textContent?.trim()).toBe(
        "La connexion a été refusée : rien n'écoute à cette adresse.",
      ),
    );
  });

  it('says how many entries it empties before it does', async () => {
    el('http-history-clear')!.click();
    await vi.waitFor(() =>
      expect(el('http-history-clear-count')?.textContent?.trim()).toBe('Effacer les 4 entrées ?'),
    );
    expect(http.callsOf('clearHistory')).toEqual([]);

    el('http-history-clear-confirm')!.click();
    await vi.waitFor(() => expect(el('http-history-empty')).not.toBeNull());
    expect(http.callsOf('clearHistory')).toHaveLength(1);
  });
});
