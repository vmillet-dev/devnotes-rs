import { RangeSetBuilder, StateEffect, StateField } from '@codemirror/state';
import { Decoration, DecorationSet, EditorView } from '@codemirror/view';
import { FieldSpan } from '@core/model/note.model';

export const setFieldMarks = StateEffect.define<readonly FieldSpan[]>();

const FIELD = Decoration.mark({ class: 'cm-field' });
const DEFAULT = Decoration.mark({ class: 'cm-field-default' });

function build(length: number, spans: readonly FieldSpan[]): DecorationSet {
  const builder = new RangeSetBuilder<Decoration>();
  for (const span of spans) {
    if (span.to > length) break;
    builder.add(span.from, span.to, FIELD);
    if (span.default) builder.add(span.default[0], span.default[1], DEFAULT);
  }
  return builder.finish();
}

/** Set whole from Rust's answer; an edit in between carries the marks with their text. */
export const fieldMarks = StateField.define<DecorationSet>({
  create: () => Decoration.none,
  update(marks, transaction) {
    for (const effect of transaction.effects) {
      if (effect.is(setFieldMarks)) return build(transaction.state.doc.length, effect.value);
    }
    return transaction.docChanged ? marks.map(transaction.changes) : marks;
  },
  provide: (field) => EditorView.decorations.from(field),
});
