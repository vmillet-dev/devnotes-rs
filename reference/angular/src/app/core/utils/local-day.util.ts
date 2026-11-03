/**
 * ⚠️ End of the local day, not midnight: a note dated today would otherwise be expired the
 * moment it is typed. Built explicitly because `new Date(value)` reads as UTC, and west of
 * Greenwich the deadline would slip back a day. `null` for anything a date input would not
 * have produced.
 */
export function endOfLocalDay(value: string): Date | null {
  const [year, month, day] = value.split('-').map(Number);
  if (!year || !month || !day) return null;

  return new Date(year, month - 1, day, 23, 59, 59, 999);
}

/** The `yyyy-mm-dd` a date input shows, read in local time like `endOfLocalDay` writes it. */
export function toDateInputValue(date: Date): string {
  const pad = (value: number): string => String(value).padStart(2, '0');
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}
