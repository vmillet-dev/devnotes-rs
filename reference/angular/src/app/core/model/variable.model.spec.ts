import { describe, expect, it } from 'vitest';
import { duplicateNames, isVariableName, toVariableRecord } from './variable.model';

describe('isVariableName', () => {
  it('accepts what a {{field}} token can be named', () => {
    expect(isVariableName('host')).toBe(true);
    expect(isVariableName('db-1_x')).toBe(true);
  });

  it('refuses what could never designate a token', () => {
    expect(isVariableName('')).toBe(false);
    expect(isVariableName('user.name')).toBe(false);
    expect(isVariableName('mon nom')).toBe(false);
  });
});

describe('toVariableRecord', () => {
  it('keeps only the rows that could be written', () => {
    const record = toVariableRecord([
      { name: 'host', value: 'db.internal' },
      { name: '', value: 'orpheline' },
      { name: 'user.name', value: 'x' },
      { name: 'port', value: '' },
    ]);

    expect(record).toEqual({ host: 'db.internal' });
  });

  it('lets the last row win, like the JSON object it becomes', () => {
    const record = toVariableRecord([
      { name: 'host', value: 'premier' },
      { name: 'host', value: 'dernier' },
    ]);

    expect(record).toEqual({ host: 'dernier' });
  });
});

describe('duplicateNames', () => {
  it('names what the panel has to warn about', () => {
    const duplicates = duplicateNames([
      { name: 'host', value: 'a' },
      { name: 'host', value: 'b' },
      { name: 'port', value: 'c' },
    ]);

    expect([...duplicates]).toEqual(['host']);
  });

  it('does not call two unnamed rows a duplicate', () => {
    expect(
      duplicateNames([
        { name: '', value: '' },
        { name: '', value: '' },
      ]).size,
    ).toBe(0);
  });
});
