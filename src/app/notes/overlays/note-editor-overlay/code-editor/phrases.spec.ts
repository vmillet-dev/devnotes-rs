import { describe, expect, it } from 'vitest';
import { phrasesFrom } from './phrases';

describe('phrasesFrom', () => {
  it("puts each translation under CodeMirror's own word for it", () => {
    expect(
      phrasesFrom({ find: 'Rechercher', replaceOne: 'remplacer', replaceAll: 'tout remplacer' }),
    ).toEqual({
      Find: 'Rechercher',
      replace: 'remplacer',
      'replace all': 'tout remplacer',
    });
  });

  it('leaves a word missing or empty to its English', () => {
    expect(phrasesFrom({ next: '', previous: 42 })).toEqual({});
  });
});
