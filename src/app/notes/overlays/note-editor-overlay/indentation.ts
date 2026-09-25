import { LanguageTag } from '@core/model/language.model';
import { IndentChoice } from '@core/services/settings/app-settings.model';

/** Four spaces by their own style guides; Go is `gofmt`'s tab, and the rest two spaces. */
const FOUR_SPACES: ReadonlySet<LanguageTag> = new Set(['py', 'rs', 'java', 'cs', 'php', 'c']);

/** How far Shift+Tab takes back a line indented with spaces, when a level is a tab. */
const TAB_WIDTH = 4;

/** One level of indentation: the user's choice, or the note's language's habit. */
export function indentUnit(choice: IndentChoice, language: LanguageTag): string {
  switch (choice) {
    case 'two-spaces':
      return '  ';
    case 'four-spaces':
      return '    ';
    case 'tab':
      return '\t';
    case 'language':
      return language === 'go' ? '\t' : FOUR_SPACES.has(language) ? '    ' : '  ';
  }
}

/** One replacement, so the field's undo takes it back in one step. */
export interface TextEdit {
  readonly from: number;
  readonly to: number;
  readonly insert: string;
  readonly selectionStart: number;
  readonly selectionEnd: number;
}

/**
 * Tab. At a caret, spaces up to the next level (or a tab); with a selection, one more level
 * on every line it touches — never in place of the selected text.
 */
export function indent(text: string, start: number, end: number, unit: string): TextEdit {
  if (start === end) {
    const column = start - lineStart(text, start);
    const insert = unit === '\t' ? unit : ' '.repeat(unit.length - (column % unit.length));
    const caret = start + insert.length;
    return { from: start, to: end, insert, selectionStart: caret, selectionEnd: caret };
  }

  return editLines(text, start, end, (line) => ({ remove: 0, add: line === '' ? '' : unit }));
}

/** Shift+Tab: one level less on every line the caret or the selection touches, if it has one. */
export function outdent(text: string, start: number, end: number, unit: string): TextEdit | null {
  const width = unit === '\t' ? TAB_WIDTH : unit.length;
  const edit = editLines(text, start, end, (line) => {
    if (line.startsWith('\t')) return { remove: 1, add: '' };
    const spaces = /^ */.exec(line)?.[0].length ?? 0;
    return { remove: Math.min(spaces, width), add: '' };
  });

  return edit.insert === text.slice(edit.from, edit.to) ? null : edit;
}

/** Through the editing commands, which is what keeps the indentation in the field's undo. */
export function applyEdit(field: HTMLTextAreaElement, edit: TextEdit): void {
  field.setSelectionRange(edit.from, edit.to);
  document.execCommand(edit.insert ? 'insertText' : 'delete', false, edit.insert);
  field.setSelectionRange(edit.selectionStart, edit.selectionEnd);
}

interface LineChange {
  /** Characters taken from the start of the line. */
  readonly remove: number;
  /** Put at its start in their place. */
  readonly add: string;
}

function lineStart(text: string, at: number): number {
  return text.lastIndexOf('\n', at - 1) + 1;
}

/** A selection ending at the start of a line leaves that line alone, as in any code editor. */
function editLines(text: string, start: number, end: number, change: (line: string) => LineChange): TextEdit {
  const from = lineStart(text, start);
  const last = end > start && text[end - 1] === '\n' ? end - 1 : end;
  const next = text.indexOf('\n', last);
  const to = next === -1 ? text.length : next;

  let oldAt = from;
  let newAt = from;
  let selectionStart = start;
  let selectionEnd = end;
  const lines = text
    .slice(from, to)
    .split('\n')
    .map((line) => {
      const { remove, add } = change(line);
      const place = (at: number): number => newAt + add.length + Math.max(0, at - oldAt - remove);
      if (start >= oldAt && start <= oldAt + line.length) {
        // A selection from a line's start keeps the level it just gained.
        selectionStart = start === oldAt && start !== end ? newAt : place(start);
      }
      if (end >= oldAt && end <= oldAt + line.length) selectionEnd = place(end);

      const changed = add + line.slice(remove);
      oldAt += line.length + 1;
      newAt += changed.length + 1;
      return changed;
    });
  const insert = lines.join('\n');
  if (end > to) selectionEnd = end + insert.length - (to - from);

  return { from, to, insert, selectionStart, selectionEnd };
}
