import { describe, expect, it } from 'vitest';
import { endOfLocalDay, toDateInputValue } from './local-day.util';

describe('endOfLocalDay', () => {
  it('lands on the last millisecond of that day, in local time', () => {
    const at = endOfLocalDay('2026-03-14');

    expect(at?.getFullYear()).toBe(2026);
    expect(at?.getMonth()).toBe(2);
    expect(at?.getDate()).toBe(14);
    expect(at?.getHours()).toBe(23);
    expect(at?.getMilliseconds()).toBe(999);
  });

  it('refuses what a date input would not have produced', () => {
    expect(endOfLocalDay('pas-une-date')).toBeNull();
    expect(endOfLocalDay('')).toBeNull();
  });
});

describe('toDateInputValue', () => {
  it('reads back what endOfLocalDay wrote', () => {
    expect(toDateInputValue(endOfLocalDay('2026-03-04') as Date)).toBe('2026-03-04');
  });
});
