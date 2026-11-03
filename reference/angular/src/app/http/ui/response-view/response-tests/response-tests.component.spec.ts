import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { TestResult } from '@core/model/http.model';
import { provideAppTesting } from '@testing/testing.providers';
import { ResponseTestsComponent } from './response-tests.component';

const passed = (name: string, extra: Partial<TestResult> = {}): TestResult => ({
  name,
  passed: true,
  skipped: false,
  level: 'request',
  ...extra,
});
const failed = (name: string, extra: Partial<TestResult> = {}): TestResult => ({
  name,
  passed: false,
  skipped: false,
  level: 'request',
  ...extra,
});

describe('ResponseTestsComponent', () => {
  let fixture: ComponentFixture<ResponseTestsComponent>;
  let opened: TestResult[];

  beforeEach(() => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({ imports: [ResponseTestsComponent], providers: [provideAppTesting()] });
  });

  async function render(tests: readonly TestResult[], openable = true) {
    fixture = TestBed.createComponent(ResponseTestsComponent);
    fixture.componentRef.setInput('tests', tests);
    fixture.componentRef.setInput('requestName', 'Payer');
    fixture.componentRef.setInput('openable', openable);
    opened = [];
    fixture.componentInstance.lineOpened.subscribe((test) => opened.push(test));
    fixture.autoDetectChanges();
    await fixture.whenStable();
  }

  const root = () => fixture.nativeElement as HTMLElement;
  const text = (testId: string) =>
    root().querySelector(`[data-testid="${testId}"]`)?.textContent?.replace(/\s+/g, ' ').trim() ?? null;
  const all = (testId: string) =>
    [...root().querySelectorAll(`[data-testid="${testId}"]`)].map((node) =>
      node.textContent?.replace(/\s+/g, ' ').trim(),
    );

  it('counts the failures, groups the tests by whose they are, and says what was expected', async () => {
    await render([
      passed('Le statut est 200 ou 402'),
      failed('Le montant est débité en euros', {
        expected: '"eur"',
        actual: '"usd"',
        line: 8,
        code: "pm.expect(res.json().currency).to.equal('eur');",
      }),
      failed('La réponse porte un identifiant de trace', {
        level: 'collection',
        holder: 'API Paiements',
        owner: 'api',
        message: "expected {} to have property 'x-request-id'",
      }),
      { name: 'Plus tard', passed: false, skipped: true },
    ]);

    expect(text('http-tests-failed')).toBe('2 échecs');
    expect(text('http-tests-ratio')).toBe('1/3 tests');
    expect(all('http-tests-group')).toEqual(['Payer', 'API Paiements · tests de la collection']);
    expect(all('http-test-versus')).toHaveLength(1);
    expect(root().querySelector('.expected')?.textContent).toBe('"eur"');
    expect(root().querySelector('.actual')?.textContent).toBe('"usd"');
    expect(text('http-test-message')).toBe("expected {} to have property 'x-request-id'");
    expect(text('http-test-code')).toBe("pm.expect(res.json().currency).to.equal('eur');");
    const results = [...root().querySelectorAll('[data-testid="http-test"]')].map((test) =>
      test.getAttribute('data-result'),
    );
    expect(results).toEqual(['passed', 'failed', 'skipped', 'failed']);

    root().querySelector<HTMLButtonElement>('[data-testid="http-test-line"]')!.click();
    expect(opened.map((test) => test.line)).toEqual([8]);
  });

  it('says all pass, and leads to no line from the history', async () => {
    await render([passed('a'), failed('b', { level: 'folder', holder: 'Factures', line: 3 })], false);
    expect(all('http-tests-group')).toEqual(['Payer', 'Factures · tests du dossier']);
    expect(text('http-test-line')).toBeNull();

    await render([passed('a'), passed('b')]);
    expect(text('http-tests-passed')).toBe('Tout passe');
    expect(text('http-tests-ratio')).toBe('2/2 tests');
  });
});
