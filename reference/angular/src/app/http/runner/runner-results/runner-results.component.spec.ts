import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { RunResult, RunnerFilter } from '@core/model/http-runner.model';
import { sentResponse } from '@testing/fake-http-repository';
import { provideAppTesting } from '@testing/testing.providers';
import { RunnerResultsComponent } from './runner-results.component';

function result(name: string, overrides: Partial<RunResult> = {}): RunResult {
  return {
    iteration: 0,
    requestId: name,
    name,
    path: ['Factures'],
    kind: 'http',
    method: 'POST',
    status: 200,
    reason: 'OK',
    millis: 88,
    tests: [{ name: 'statut', passed: true, skipped: false }],
    error: null,
    stop: null,
    skipped: false,
    response: null,
    ...overrides,
  };
}

describe('RunnerResultsComponent', () => {
  let fixture: ComponentFixture<RunnerResultsComponent>;
  let filters: RunnerFilter[];

  beforeEach(async () => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({ imports: [RunnerResultsComponent], providers: [provideAppTesting()] });
    fixture = TestBed.createComponent(RunnerResultsComponent);
    fixture.componentRef.setInput('results', [
      result('Login', { path: [], response: sentResponse() }),
      result('Payer', {
        status: 402,
        reason: 'Payment Required',
        tests: [
          { name: 'statut', passed: true, skipped: false },
          {
            name: 'devise',
            passed: false,
            skipped: false,
            expected: '"eur"',
            actual: '"usd"',
            code: "pm.expect(c).to.equal('eur');",
          },
        ],
      }),
      result('Reçu', { iteration: 1, status: null, tests: [], error: { code: 'httpRefused', params: {} } }),
      result('Annuler', { iteration: 1, skipped: true, tests: [] }),
      result('Stop', {
        iteration: 1,
        tests: [{ name: 'muet', passed: false, skipped: false, message: 'boom' }],
        stop: { kind: 'error', message: 'arrêt' },
      }),
    ]);
    fixture.componentRef.setInput('iterations', 2);
    fixture.componentRef.setInput('data', {
      columns: ['customer'],
      rows: [{ customer: 'cus_1' }, { customer: 'cus_2' }],
    });
    fixture.componentRef.setInput('millis', 2400);
    fixture.componentRef.setInput('filter', 'all');
    filters = [];
    fixture.componentInstance.filterChanged.subscribe((filter) => filters.push(filter));
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  const root = () => fixture.nativeElement as HTMLElement;
  const text = (testId: string) =>
    root().querySelector(`[data-testid="${testId}"]`)?.textContent?.replace(/\s+/g, ' ').trim();
  const all = (testId: string) => [...root().querySelectorAll<HTMLElement>(`[data-testid="${testId}"]`)];

  it('sums the run up, and groups its requests by iteration with the row each read', () => {
    expect(text('http-runner-summary')).toContain('3 échecs');
    expect(text('http-runner-figures')).toBe('5 requêtes · 2/4 tests · 2,4 s');
    expect(
      all('http-runner-iteration').map((group) => group.textContent?.replace(/\s+/g, ' ').trim()),
    ).toEqual(['Itération 1 / 2 customer = cus_1', 'Itération 2 / 2 customer = cus_2']);
    expect(all('http-runner-result').map((row) => row.dataset['result'])).toEqual([
      'passed',
      'failed',
      'failed',
      'skipped',
      'failed',
    ]);
    expect(all('http-runner-result')[1]?.textContent).toContain('402 Payment Required');
    expect(all('http-runner-result')[2]?.textContent).toContain('Pas de réponse');
    expect(all('http-runner-result')[3]?.textContent).toContain('Écartée par un script');
  });

  it('opens a failed request on its assertions, and any other on a click', async () => {
    expect(all('http-runner-details')).toHaveLength(3);
    expect(all('http-runner-details')[0]?.textContent).toContain('attendu "eur", reçu "usd"');
    expect(all('http-runner-details')[0]?.textContent).toContain("pm.expect(c).to.equal('eur');");
    expect(all('http-runner-details')[1]?.textContent).toContain('La connexion a été refusée');
    expect(all('http-runner-details')[2]?.textContent).toContain('arrêt');
    expect(all('http-runner-details')[2]?.textContent).toContain('boom');

    all('http-runner-result')[0]!.querySelector<HTMLButtonElement>('.top')!.click();
    await fixture.whenStable();
    expect(root().querySelector('app-response-view')).not.toBeNull();
    all('http-runner-result')[1]!.querySelector<HTMLButtonElement>('.top')!.click();
    await fixture.whenStable();
    expect(all('http-runner-details')).toHaveLength(3);
  });

  it('filters by what passed and failed', async () => {
    all('http-runner-filter')
      .find((button) => button.dataset['filter'] === 'failed')!
      .click();
    expect(filters).toEqual(['failed']);
    fixture.componentRef.setInput('filter', 'failed');
    await fixture.whenStable();
    expect(all('http-runner-result')).toHaveLength(3);
    fixture.componentRef.setInput('filter', 'passed');
    await fixture.whenStable();
    expect(all('http-runner-result').map((row) => row.dataset['result'])).toEqual(['passed', 'skipped']);
  });

  it('says a cancelled run was cancelled and how far it went, never that all pass', async () => {
    fixture.componentRef.setInput('results', [result('Login'), result('Payer')]);
    fixture.componentRef.setInput('cancelledOf', 10);
    await fixture.whenStable();

    expect(text('http-runner-cancelled')).toBe('Annulé · 2 sur 10 requêtes');
    expect(text('http-runner-summary')).not.toContain('Tout passe');
  });

  it('says all pass with no bar when nothing was tested, for a single iteration', async () => {
    fixture.componentRef.setInput('results', [result('Login', { tests: [] })]);
    fixture.componentRef.setInput('iterations', 1);
    fixture.componentRef.setInput('millis', null);
    await fixture.whenStable();
    expect(text('http-runner-summary')).toContain('Tout passe');
    expect(root().querySelector('.bar')).toBeNull();
    expect(all('http-runner-iteration')).toEqual([]);
  });
});
