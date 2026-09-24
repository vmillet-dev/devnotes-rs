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
