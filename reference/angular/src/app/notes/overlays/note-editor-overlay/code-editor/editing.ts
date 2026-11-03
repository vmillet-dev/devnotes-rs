import { isolateHistory } from '@codemirror/commands';
import { indentUnit } from '@codemirror/language';
import type { Command, EditorView, KeyBinding } from '@codemirror/view';
import { TextEdit, indent, outdent } from '../indentation';

/** One transaction, so one undo takes it back; `apart` keeps it from the typing either side. */
export function applyEdit(view: EditorView, edit: TextEdit, userEvent: string, apart = false): void {
  view.dispatch({
    changes: { from: edit.from, to: edit.to, insert: edit.insert },
    selection: { anchor: edit.selectionStart, head: edit.selectionEnd },
    scrollIntoView: true,
    userEvent,
    annotations: apart ? isolateHistory.of('full') : [],
  });
}

function shiftLevel(less: boolean): Command {
  return (view) => {
    const { from, to } = view.state.selection.main;
    const text = view.state.doc.toString();
    const unit = view.state.facet(indentUnit);
    const edit = less ? outdent(text, from, to, unit) : indent(text, from, to, unit);
    if (edit) applyEdit(view, edit, less ? 'delete.dedent' : 'input.indent');
    // Handled even when nothing moved: Tab stays in the text, Ctrl+M is the way out.
    return true;
  };
}

/** The textarea's Tab and Shift+Tab, rules and all: the user's indentation, a level at a time. */
export const TAB_KEYS: readonly KeyBinding[] = [
  { key: 'Tab', run: shiftLevel(false), shift: shiftLevel(true) },
];
