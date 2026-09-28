import { describe, expect, it } from 'vitest';
import { countWords } from './word-count';

describe('countWords', () => {
  it('counts the words and none of the Markdown around them', () => {
    expect(countWords('## Standup\n\n- **Ship** it and `tag`\n> quoted _here_')).toBe(7);
  });

  it('reads a link as its words, not its address', () => {
    expect(countWords('See [the runbook](https://example.com/run-book).')).toBe(3);
  });

  it('reads neither a task box nor a tab written as an entity as a word', () => {
    expect(countWords('- [x] ship it\n- [ ] tag it\n&#9;indented')).toBe(5);
  });

  it('keeps a word whole across an apostrophe or a hyphen', () => {
    expect(countWords("l’équipe a re-déployé l'API")).toBe(4);
  });

  it('reads table cells as words and their borders as nothing', () => {
    expect(countWords('| Host | Port |\n| --- | --- |\n| db | 5432 |')).toBe(4);
  });

  it('says zero for an empty note', () => {
    expect(countWords('')).toBe(0);
    expect(countWords('  \n\n  ')).toBe(0);
  });
});
