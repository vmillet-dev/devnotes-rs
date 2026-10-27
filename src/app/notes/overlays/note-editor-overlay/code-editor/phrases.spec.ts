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

  it('gives a label its colon back, the space before it kept for CodeMirror to follow', () => {
    expect(phrasesFrom({ goToLine: 'Aller à la ligne :' })['Go to line']).toBe('Aller à la ligne ');
    expect(phrasesFrom({ goToLine: 'Go to line:' })['Go to line']).toBe('Go to line');
  });

  it('leaves a word missing or empty to its English', () => {
    expect(phrasesFrom({ go: '', close: 42 })).toEqual({});
  });
});
