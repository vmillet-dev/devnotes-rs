import { describe, expect, it } from 'vitest';
import { TextEdit, indent, indentUnit, outdent } from './indentation';

/** The text after the edit, with `[` and `]` where the selection lands. */
function applied(text: string, edit: TextEdit | null): string | null {
  if (!edit) return null;
  const after = text.slice(0, edit.from) + edit.insert + text.slice(edit.to);
  return `${after.slice(0, edit.selectionStart)}[${after.slice(edit.selectionStart, edit.selectionEnd)}]${after.slice(edit.selectionEnd)}`;
}

/** `[` and `]` mark the selection in the text before the edit. */
function selected(marked: string): [string, number, number] {
  const start = marked.indexOf('[');
  const end = marked.indexOf(']') - 1;
  return [marked.replace('[', '').replace(']', ''), start, end];
}

function tab(marked: string, unit = '  '): string | null {
  const [text, start, end] = selected(marked);
  return applied(text, indent(text, start, end, unit));
}

function shiftTab(marked: string, unit = '  '): string | null {
  const [text, start, end] = selected(marked);
  return applied(text, outdent(text, start, end, unit));
}

describe('indentUnit', () => {
  it.each([
    ['py', '    '],
    ['rs', '    '],
    ['java', '    '],
    ['cs', '    '],
    ['php', '    '],
    ['c', '    '],
    ['go', '\t'],
    ['ts', '  '],
    ['yml', '  '],
    ['sh', '  '],
  ] as const)('follows %s by default', (language, unit) => {
    expect(indentUnit('language', language)).toBe(unit);
  });

  it('holds to the choice the user made, whatever the language', () => {
    expect(indentUnit('two-spaces', 'py')).toBe('  ');
    expect(indentUnit('four-spaces', 'yml')).toBe('    ');
    expect(indentUnit('tab', 'rs')).toBe('\t');
  });
});

describe('indent', () => {
  it('puts spaces up to the next level at the caret', () => {
    expect(tab('if x:\n[]return')).toBe('if x:\n  []return');
    expect(tab('a[]b', '    ')).toBe('a   []b');
  });

  it('puts a tab at the caret when a level is one', () => {
    expect(tab('func[]', '\t')).toBe('func\t[]');
  });

  it('adds a level to every line a selection touches, and keeps them selected', () => {
    expect(tab('a\n[b\nc]\nd')).toBe('a\n[  b\n  c]\nd');
  });

  it('adds a level to the line of a selection inside it, rather than typing over the words', () => {
    expect(tab('let [x] = 1')).toBe('  let [x] = 1');
  });

  it('leaves out the line a selection ends at the start of, and the empty lines', () => {
    expect(tab('[a\n\nb\n]c')).toBe('[  a\n\n  b\n]c');
  });
});

describe('outdent', () => {
  it('takes one level off the line of the caret', () => {
    expect(shiftTab('    ret[]urn')).toBe('  ret[]urn');
  });

  it('takes a tab off before spaces', () => {
    expect(shiftTab('\t  x[]', '    ')).toBe('  x[]');
  });

  it('takes what indentation there is when it is less than a level', () => {
    expect(shiftTab(' x[]', '    ')).toBe('x[]');
  });

  it('takes up to four spaces when a level is a tab', () => {
    expect(shiftTab('      x[]', '\t')).toBe('  x[]');
  });

  it('takes a level off every line a selection touches', () => {
    expect(shiftTab('[  a\n    b\nc]')).toBe('[a\n  b\nc]');
  });

  it('keeps a caret inside the indentation it removes at the line start', () => {
    expect(shiftTab('  [] x')).toBe('[] x');
  });

  it('changes nothing on lines with no indentation', () => {
    expect(shiftTab('a\n[b]')).toBeNull();
  });
});
