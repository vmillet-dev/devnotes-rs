import { describe, expect, it } from 'vitest';
import { nameRank, rankedByName } from './fold.util';

describe('nameRank', () => {
  it('ranks a name by how it answers the search, case and accents aside', () => {
    expect(nameRank('JSON', 'json')).toBe(0);
    expect(nameRank('JSON ↔ YAML', 'json')).toBe(1);
    expect(nameRank('Connexion psql', 'PSQL')).toBe(2);
    expect(nameRank('Lancerpsql', 'psql')).toBe(3);
    expect(nameRank('Diff', 'json')).toBe(4);
    expect(nameRank('Écart', 'ecart')).toBe(0);
  });
});

describe('rankedByName', () => {
  it('puts the best names first, the order given breaking ties', () => {
    const tools = ['Diff', 'JWT', 'JSON ↔ YAML', 'JSON'];

    expect(rankedByName(tools, 'json', (name) => name)).toEqual(['JSON', 'JSON ↔ YAML', 'Diff', 'JWT']);
  });
});
