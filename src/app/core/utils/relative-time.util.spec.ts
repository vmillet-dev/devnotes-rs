import { describe, expect, it } from 'vitest';
import { expiryRef, relativeTimeRef, spanRef } from './relative-time.util';

describe('relativeTimeRef', () => {
  it('returns the "just now" key when under a minute has elapsed', () => {
    const now = new Date('2026-01-01T12:00:00Z');
    const date = new Date('2026-01-01T11:59:45Z');

    expect(relativeTimeRef(date, now)).toEqual({ key: 'time.justNow' });
  });

  it('returns the minutes key when under an hour has elapsed', () => {
    const now = new Date('2026-01-01T12:00:00Z');
    const date = new Date('2026-01-01T11:45:00Z');

    expect(relativeTimeRef(date, now)).toEqual({ key: 'time.minutesAgo', params: { count: 15 } });
  });

  it('returns the hours key when under a day has elapsed', () => {
    const now = new Date('2026-01-01T12:00:00Z');
    const date = new Date('2026-01-01T09:00:00Z');

    expect(relativeTimeRef(date, now)).toEqual({ key: 'time.hoursAgo', params: { count: 3 } });
  });

  it('returns the days key when a day or more has elapsed', () => {
    const now = new Date('2026-01-05T12:00:00Z');
    const date = new Date('2026-01-01T12:00:00Z');

    expect(relativeTimeRef(date, now)).toEqual({ key: 'time.daysAgo', params: { count: 4 } });
  });
});

describe('expiryRef', () => {
  it('returns the "expired" key when the expiry date is in the past', () => {
    const now = new Date('2026-01-05T12:00:00Z');
    const at = new Date('2026-01-01T12:00:00Z');

    expect(expiryRef(at, now)).toEqual({ key: 'time.expired' });
  });

  it('returns the "expired" key when the expiry date is exactly now', () => {
    const now = new Date('2026-01-05T12:00:00Z');

    expect(expiryRef(now, now)).toEqual({ key: 'time.expired' });
  });

  /** MET-07: today + 3 at 21:45 read « expire dans 4j ». */
  it('counts the local days to the expiry day, whatever the hour', () => {
    const evening = new Date(2026, 9, 21, 21, 45);
    const morning = new Date(2026, 9, 21, 8, 0);
    const endOf = (day: number) => new Date(2026, 9, day, 23, 59, 59, 999);

    expect(expiryRef(endOf(24), evening)).toEqual({ key: 'time.expiresIn', params: { count: 3 } });
    expect(expiryRef(endOf(24), morning)).toEqual({ key: 'time.expiresIn', params: { count: 3 } });
    expect(expiryRef(endOf(22), evening)).toEqual({ key: 'time.expiresIn', params: { count: 1 } });
    expect(expiryRef(endOf(21), evening)).toEqual({ key: 'time.expiresToday' });
  });

  it('returns the days-remaining key when the expiry date is in the future', () => {
    const now = new Date('2026-01-01T12:00:00Z');
    const at = new Date('2026-01-04T12:00:00Z');

    expect(expiryRef(at, now)).toEqual({ key: 'time.expiresIn', params: { count: 3 } });
  });
});

describe('spanRef', () => {
  const now = new Date('2026-09-29T12:00:00Z');
  const at = (iso: string) => new Date(iso).getTime();

  it('says under a minute either way without counting seconds', () => {
    expect(spanRef(at('2026-09-29T11:59:30Z'), now)).toEqual({ key: 'time.span.now' });
    expect(spanRef(at('2026-09-29T12:00:59Z'), now)).toEqual({ key: 'time.span.now' });
  });

  it('counts the largest whole unit, in the past and in the future', () => {
    expect(spanRef(at('2026-09-29T11:15:00Z'), now)).toEqual({
      key: 'time.span.past.minutes',
      params: { count: 45 },
    });
    expect(spanRef(at('2026-09-30T14:00:00Z'), now)).toEqual({
      key: 'time.span.future.days',
      params: { count: 1 },
    });
    expect(spanRef(at('2026-07-01T12:00:00Z'), now)).toEqual({
      key: 'time.span.past.months',
      params: { count: 2 },
    });
    expect(spanRef(at('1970-01-01T00:00:00Z'), now)).toEqual({
      key: 'time.span.past.years',
      params: { count: 56 },
    });
    expect(spanRef(at('2038-01-19T03:14:07Z'), now)).toEqual({
      key: 'time.span.future.years',
      params: { count: 11 },
    });
    expect(spanRef(at('2026-09-29T15:00:00Z'), now)).toEqual({
      key: 'time.span.future.hours',
      params: { count: 3 },
    });
  });
});
