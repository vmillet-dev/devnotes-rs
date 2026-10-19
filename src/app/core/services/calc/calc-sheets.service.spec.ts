import { TestBed } from '@angular/core/testing';
import { TranslocoService } from '@jsverse/transloco';
import { firstValueFrom } from 'rxjs';
import { describe, expect, it, vi } from 'vitest';
import { SettingsStore } from '@core/services/settings/settings.store';
import { ClockService } from '@core/services/time/clock.service';
import { FakeCalcRepository } from '@testing/fake-calc-repository';
import { provideAppTesting } from '@testing/testing.providers';
import { CalcSheets } from './calc-sheets.service';

async function setUp(): Promise<{ sheets: CalcSheets; calc: FakeCalcRepository }> {
  TestBed.resetTestingModule();
  const calc = new FakeCalcRepository();
  TestBed.configureTestingModule({ providers: [provideAppTesting({ calcRepository: calc })] });
  await firstValueFrom(TestBed.inject(TranslocoService).load('fr'));
  return { sheets: TestBed.inject(CalcSheets), calc };
}

describe('CalcSheets', () => {
  it('asks for the sheet on the local day, however late in it', async () => {
    const { sheets, calc } = await setUp();
    vi.spyOn(TestBed.inject(ClockService), 'now').mockReturnValue(new Date(2026, 0, 5, 23, 59));

    await sheets.evaluate('2 + 2');

    expect(calc.asked).toEqual([{ sheet: '2 + 2', today: '2026-01-05' }]);
  });

  it('copies a sheet with its results as comments, and nothing when Rust failed', async () => {
    const { sheets, calc } = await setUp();
    calc.sheet = {
      rows: [
        {
          line: {
            kind: 'value',
            value: { kind: 'number', value: '4', unit: null },
            name: null,
            aggregate: false,
            dimension: 'number',
          },
          spans: [],
          converting: null,
        },
      ],
      variables: [],
    };

    expect(await sheets.copyText('2 + 2')).toBe('2 + 2 // 4');

    calc.failNext = new Error('bridge down');
    expect(await sheets.copyText('2 + 2')).toBeNull();
  });

  it('writes the copy in the formats the preferences chose', async () => {
    const { sheets, calc } = await setUp();
    const settings = TestBed.inject(SettingsStore);
    settings.calcNumberFormat.write('de-DE');
    settings.calcDateFormat.write('numeric');
    settings.calcDecimals.write('3');
    calc.sheet = {
      rows: [
        {
          line: {
            kind: 'value',
            value: { kind: 'number', value: '1578.13', unit: null },
            name: null,
            aggregate: false,
            dimension: 'number',
          },
          spans: [],
          converting: null,
        },
        {
          line: {
            kind: 'value',
            value: { kind: 'date', date: '2026-12-01' },
            name: null,
            aggregate: false,
            dimension: 'date',
          },
          spans: [],
          converting: null,
        },
      ],
      variables: [],
    };

    const copied = (await sheets.copyText('1578,13\naujourd’hui'))!.split('\n');

    expect(copied[0]!.replace(/\D(?=\d{3},)/, '_')).toBe('1578,13 // 1_578,130');
    expect(copied[1]).toBe('aujourd’hui // 01.12.2026');
  });
});
