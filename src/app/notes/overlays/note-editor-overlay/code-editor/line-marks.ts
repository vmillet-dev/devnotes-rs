import { RangeSetBuilder, StateEffect, StateField, Text } from '@codemirror/state';
import { Decoration, DecorationSet, EditorView } from '@codemirror/view';

/** Zero-based lines: what the format just changed, and what the visualiser has selected. */
export interface LineMarks {
  readonly marked: ReadonlySet<number>;
  readonly focused: ReadonlySet<number>;
}

export const setLineMarks = StateEffect.define<LineMarks>();

const MARKED = Decoration.line({ class: 'cm-marked' });
const FOCUSED = Decoration.line({ class: 'cm-focused-line' });

function build(doc: Text, marks: LineMarks): DecorationSet {
  const builder = new RangeSetBuilder<Decoration>();
  const lines = [...new Set([...marks.marked, ...marks.focused])].sort((a, b) => a - b);
  for (const index of lines) {
    if (index < 0 || index >= doc.lines) continue;
    const from = doc.line(index + 1).from;
    if (marks.marked.has(index)) builder.add(from, from, MARKED);
    if (marks.focused.has(index)) builder.add(from, from, FOCUSED);
  }
  return builder.finish();
}

/** Set whole by `setLineMarks`; an edit in between carries them with their lines. */
export const lineMarks = StateField.define<DecorationSet>({
  create: () => Decoration.none,
  update(marks, transaction) {
    for (const effect of transaction.effects) {
      if (effect.is(setLineMarks)) return build(transaction.state.doc, effect.value);
    }
    return transaction.docChanged ? marks.map(transaction.changes) : marks;
  },
  provide: (field) => EditorView.decorations.from(field),
});
