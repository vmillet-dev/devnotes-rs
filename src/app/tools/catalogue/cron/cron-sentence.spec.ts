import { TestBed } from '@angular/core/testing';
import { TranslocoService } from '@jsverse/transloco';
import { firstValueFrom } from 'rxjs';
import { beforeEach, describe, expect, it } from 'vitest';
import { CronField, FieldReading, Piece } from '@core/model/tool-answers.model';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { cronSentence } from './cron-sentence';

const EVERY: Piece = { kind: 'every' };
const FIVE: readonly CronField[] = ['minute', 'hour', 'dayOfMonth', 'month', 'dayOfWeek'];

function fields(...pieces: (readonly Piece[])[]): FieldReading[] {
  const names = pieces.length === 6 ? (['second', ...FIVE] as const) : FIVE;
  return pieces.map((field, index) => ({
    field: names[index]!,
    text: '',
    start: 0,
    end: 0,
    pieces: [...field],
  }));
}

const value = (value: number): Piece => ({ kind: 'value', value });

describe('cronSentence', () => {
  let transloco: TranslocoService;

  beforeEach(async () => {
    TestBed.configureTestingModule({ providers: [provideTranslocoTesting()] });
    transloco = TestBed.inject(TranslocoService);
    await Promise.all([firstValueFrom(transloco.load('fr')), firstValueFrom(transloco.load('en'))]);
    transloco.setActiveLang('fr');
  });

  const say = (read: FieldReading[]): string => {
    const list = new Intl.ListFormat(transloco.getActiveLang(), { type: 'conjunction' });
    return cronSentence(
      read,
      (key, params) => transloco.translate(key, params),
      (items) => list.format(items),
    );
  };

  it('reads the ticket’s own example as it asks', () => {
    const read = fields(
      [{ kind: 'step', step: 15 }],
      [{ kind: 'range', from: 9, to: 18 }],
      [EVERY],
      [EVERY],
      [{ kind: 'range', from: 1, to: 5 }],
    );

    expect(say(read)).toBe('Toutes les 15 minutes, de 9 h à 18 h, du lundi au vendredi');
    transloco.setActiveLang('en');
    expect(say(read)).toBe('Every 15 minutes, from 9:00 to 18:00, from Monday to Friday');
  });

  it('says every minute, and every second with a sixth field', () => {
    expect(say(fields([EVERY], [EVERY], [EVERY], [EVERY], [EVERY]))).toBe('Toutes les minutes');
    expect(say(fields([EVERY], [EVERY], [EVERY], [EVERY], [EVERY], [EVERY]))).toBe('Toutes les secondes');
    expect(say(fields([{ kind: 'step', step: 30 }], [EVERY], [EVERY], [EVERY], [EVERY], [EVERY]))).toBe(
      'Toutes les 30 secondes',
    );
  });

  it('writes plain minutes and hours as times', () => {
    expect(say(fields([value(30)], [value(9), value(18)], [EVERY], [EVERY], [EVERY]))).toBe(
      'À 09:30 et 18:30',
    );
    expect(say(fields([value(0)], [value(0)], [value(1)], [value(1)], [EVERY]))).toBe(
      'À 00:00, le 1er du mois, en janvier',
    );
  });

  it('says on the hour as every hour, over a range or a step', () => {
    expect(say(fields([value(0)], [EVERY], [EVERY], [EVERY], [EVERY]))).toBe('Toutes les heures');
    expect(say(fields([value(0)], [{ kind: 'step', step: 2 }], [EVERY], [EVERY], [EVERY]))).toBe(
      'Toutes les 2 heures',
    );
    expect(say(fields([value(0)], [{ kind: 'range', from: 9, to: 17 }], [EVERY], [EVERY], [EVERY]))).toBe(
      'De 9 h à 17 h',
    );
  });

  it('lists minutes, and steps within a range', () => {
    expect(say(fields([value(0), value(15), value(45)], [EVERY], [EVERY], [EVERY], [EVERY]))).toBe(
      'Aux minutes 0, 15 et 45',
    );
    expect(
      say(fields([{ kind: 'steppedRange', step: 10, from: 5, to: 59 }], [EVERY], [EVERY], [EVERY], [EVERY])),
    ).toBe('Toutes les 10 minutes, de la minute 5 à la minute 59');
    expect(
      say(fields([value(0), { kind: 'range', from: 10, to: 20 }], [EVERY], [EVERY], [EVERY], [EVERY])),
    ).toBe('À la minute 0 et de la minute 10 à la minute 20');
  });

  it('names the special days', () => {
    const day = (piece: Piece, field: 'dom' | 'dow') =>
      say(
        fields(
          [value(0)],
          [value(8)],
          field === 'dom' ? [piece] : [EVERY],
          [EVERY],
          field === 'dow' ? [piece] : [EVERY],
        ),
      );

    expect(day({ kind: 'lastDayOfMonth' }, 'dom')).toBe('À 08:00, le dernier jour du mois');
    expect(day({ kind: 'nearestWeekday', day: 15 }, 'dom')).toBe(
      'À 08:00, le jour ouvré le plus proche du 15',
    );
    expect(day({ kind: 'lastWeekday', weekday: 5 }, 'dow')).toBe('À 08:00, le dernier vendredi du mois');
    expect(day({ kind: 'nth', weekday: 1, nth: 2 }, 'dow')).toBe('À 08:00, le deuxième lundi du mois');
    expect(day(value(7), 'dow')).toBe('À 08:00, le dimanche');
    transloco.setActiveLang('en');
    expect(day({ kind: 'nth', weekday: 1, nth: 2 }, 'dow')).toBe(
      'At 08:00, on the second Monday of the month',
    );
  });

  it('reads steps and ranges of days and months', () => {
    expect(
      say(
        fields(
          [value(0)],
          [value(6)],
          [{ kind: 'steppedRange', step: 2, from: 1, to: 15 }],
          [{ kind: 'range', from: 3, to: 6 }],
          [EVERY],
        ),
      ),
    ).toBe('À 06:00, tous les 2 jours, du 1er au 15 du mois, de mars à juin');
    expect(
      say(fields([value(0)], [value(6)], [{ kind: 'step', step: 3 }], [{ kind: 'step', step: 2 }], [EVERY])),
    ).toBe('À 06:00, tous les 3 jours, tous les 2 mois');
    expect(say(fields([value(0)], [value(6)], [EVERY], [value(1), value(7)], [value(1), value(3)]))).toBe(
      'À 06:00, les lundi et mercredi, en janvier et juillet',
    );
  });

  it('keeps each clause for minutes and hours too many to write as times', () => {
    const minutes = [0, 10, 20, 30, 40, 50].map(value);

    expect(say(fields(minutes, [value(9), value(12)], [EVERY], [EVERY], [EVERY]))).toBe(
      'Aux minutes 0, 10, 20, 30, 40 et 50, à 9 h et 12 h',
    );
    expect(
      say(
        fields(
          [{ kind: 'range', from: 0, to: 5 }],
          [{ kind: 'steppedRange', step: 2, from: 8, to: 18 }],
          [EVERY],
          [EVERY],
          [EVERY],
        ),
      ),
    ).toBe('De la minute 0 à la minute 5, toutes les 2 heures, de 8 h à 18 h');
  });
});
