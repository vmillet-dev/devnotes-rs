import { describe, expect, it } from 'vitest';
import { MAX_VISIBLE_LINES, visibleLines } from './visible-lines';

describe('visibleLines', () => {
  it('gives each line its ending and its trailing spaces', () => {
    expect(visibleLines('a \t\r\nb\rc').lines).toEqual([
      { content: 'a', trailing: ' \t', ending: 'crlf' },
      { content: 'b', trailing: '', ending: 'cr' },
      { content: 'c', trailing: '', ending: null },
    ]);
  });

  it('draws no empty line after a final newline', () => {
    expect(visibleLines('a\n').lines).toEqual([{ content: 'a', trailing: '', ending: 'lf' }]);
    expect(visibleLines('a\n\n').lines).toHaveLength(2);
  });

  it('counts what it does not draw', () => {
    const view = visibleLines('x\n'.repeat(MAX_VISIBLE_LINES + 5));

    expect(view.lines).toHaveLength(MAX_VISIBLE_LINES);
    expect(view.hidden).toBe(5);
  });
});
