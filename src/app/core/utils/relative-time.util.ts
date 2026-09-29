import { TranslationRef } from '@core/services/i18n/translation-ref.model';

const MS_PER_DAY = 24 * 60 * 60 * 1000;

/** `now` is a parameter: an internal `new Date()` would freeze the caller's `computed`. */
export function relativeTimeRef(date: Date, now: Date): TranslationRef {
  const minutes = Math.round((now.getTime() - date.getTime()) / 60_000);
  if (minutes < 1) return { key: 'time.justNow' };
  if (minutes < 60) return { key: 'time.minutesAgo', params: { count: minutes } };
  const hours = Math.round(minutes / 60);
  if (hours < 24) return { key: 'time.hoursAgo', params: { count: hours } };
  const days = Math.round(hours / 24);
  return { key: 'time.daysAgo', params: { count: days } };
}

export function expiryRef(at: Date, now: Date): TranslationRef {
  const days = Math.ceil((at.getTime() - now.getTime()) / MS_PER_DAY);
  if (days <= 0) return { key: 'time.expired' };
  return { key: 'time.expiresIn', params: { count: days } };
}

const SPANS: readonly (readonly [unit: string, ms: number])[] = [
  ['years', 365.2425 * MS_PER_DAY],
  ['months', 30.436875 * MS_PER_DAY],
  ['days', MS_PER_DAY],
  ['hours', 3_600_000],
  ['minutes', 60_000],
];

/**
 * "il y a 3 ans", "dans 2 jours": either way in time, in the largest unit the gap holds a whole
 * one of. Under a minute it says so rather than count seconds a 30-second clock cannot follow.
 */
export function spanRef(at: number, now: Date): TranslationRef {
  const gap = at - now.getTime();
  const span = SPANS.find(([, ms]) => Math.abs(gap) >= ms);
  if (span === undefined) return { key: 'time.span.now' };
  const [unit, ms] = span;
  return {
    key: `time.span.${gap < 0 ? 'past' : 'future'}.${unit}`,
    params: { count: Math.floor(Math.abs(gap) / ms) },
  };
}
