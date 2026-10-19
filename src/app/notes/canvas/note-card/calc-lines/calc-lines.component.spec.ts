import { TestBed } from '@angular/core/testing';
import { TranslocoService } from '@jsverse/transloco';
import { firstValueFrom } from 'rxjs';
import { describe, expect, it } from 'vitest';
import { SettingsStore } from '@core/services/settings/settings.store';
import { provideAppTesting } from '@testing/testing.providers';
import { CalcLinesComponent } from './calc-lines.component';

/** The narrow spaces French puts before a unit, read as plain ones. */
const SPACES = /[\s\u00a0\u202f]+/g;

describe('CalcLinesComponent', () => {
  it('shows each line written and its result, a rule above a total', async () => {
    TestBed.configureTestingModule({ imports: [CalcLinesComponent], providers: [provideAppTesting()] });
    await firstValueFrom(TestBed.inject(TranslocoService).load('fr'));
    const fixture = TestBed.createComponent(CalcLinesComponent);
    fixture.componentRef.setInput('lines', [
      {
        expression: 'domaines = 4,99 €',
        value: { kind: 'number', value: '4.99', unit: { unit: 'euro', per: null } },
        aggregate: false,
      },
      {
        expression: 'mensuel = total',
        value: { kind: 'number', value: '131.511', unit: { unit: 'euro', per: null } },
        aggregate: true,
      },
    ]);
    fixture.autoDetectChanges();
    await fixture.whenStable();

    const lines: HTMLElement[] = [
      ...fixture.nativeElement.querySelectorAll('[data-testid="note-card-calc-line"]'),
    ];
    const read = (line: HTMLElement): string =>
      [...line.children].map((part) => part.textContent!.replace(SPACES, ' ').trim()).join(' | ');
    expect(lines.map(read)).toEqual(['domaines = 4,99 € | 4,99 €', 'mensuel = total | ≈ 131,51 €']);
    expect(lines.map((line) => line.classList.contains('total'))).toEqual([false, true]);
  });

  it('writes its results in the formats the preferences chose', async () => {
    TestBed.configureTestingModule({ imports: [CalcLinesComponent], providers: [provideAppTesting()] });
    await firstValueFrom(TestBed.inject(TranslocoService).load('fr'));
    const settings = TestBed.inject(SettingsStore);
    settings.calcNumberFormat.write('en-US');
    settings.calcDateFormat.write('long');
    const fixture = TestBed.createComponent(CalcLinesComponent);
    fixture.componentRef.setInput('lines', [
      { expression: '4,5', value: { kind: 'number', value: '4.5', unit: null }, aggregate: false },
      { expression: 'demain', value: { kind: 'date', date: '2026-12-01' }, aggregate: false },
    ]);
    fixture.autoDetectChanges();
    await fixture.whenStable();

    const results = [...fixture.nativeElement.querySelectorAll('[data-testid="note-card-calc-line"]')].map(
      (line: HTMLElement) => line.lastElementChild!.textContent!.replace(SPACES, ' ').trim(),
    );
    expect(results[0]).toBe('4.5');
    expect(results[1]).toMatch(/^Tuesday,? December 1,? 2026$/);
  });
});
