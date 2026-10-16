import { TestBed } from '@angular/core/testing';
import { TranslocoService } from '@jsverse/transloco';
import { firstValueFrom } from 'rxjs';
import { describe, expect, it } from 'vitest';
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
});
