import { describe, expect, it } from 'vitest';
import { phrasesFrom } from './phrases';

describe('phrasesFrom', () => {
  it("puts each translation under CodeMirror's own word for it", () => {
    expect(phrasesFrom({ goToLine: 'Aller à la ligne', go: 'aller', close: 'fermer' })).toEqual({
      'Go to line': 'Aller à la ligne',
      go: 'aller',
      close: 'fermer',
    });
  });

  it('leaves a word missing or empty to its English', () => {
    expect(phrasesFrom({ go: '', close: 42 })).toEqual({});
  });
});
