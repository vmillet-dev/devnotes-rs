import { describe, expect, it } from 'vitest';
import { checklistProgress, noteCopyText } from './checklist.model';

describe('checklistProgress', () => {
  it('counts what is ticked', () => {
    expect(
      checklistProgress([
        { text: 'a', done: true },
        { text: 'b', done: false },
        { text: 'c', done: true },
      ]),
    ).toEqual({ done: 2, total: 3, percent: 67 });
  });

  it('reads an empty list as nothing done rather than everything done', () => {
    expect(checklistProgress([])).toEqual({ done: 0, total: 0, percent: 0 });
  });

  it('reaches exactly 100 when every task is ticked', () => {
    expect(checklistProgress([{ text: 'a', done: true }]).percent).toBe(100);
  });
});

describe('noteCopyText', () => {
  it('copies the body of an ordinary note', () => {
    expect(noteCopyText({ content: 'select 1', copyText: null })).toBe('select 1');
  });

  it('copies the rendered list of a checklist, which has no body', () => {
    expect(noteCopyText({ content: '', copyText: '- [ ] Relire' })).toBe('- [ ] Relire');
  });
});
