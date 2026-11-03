import { RangeSetBuilder, StateEffect, StateField, Text } from '@codemirror/state';
import { Decoration, DecorationSet, EditorView, WidgetType } from '@codemirror/view';
import type { CalcError, CalcSheet } from '@core/model/calc.model';

/** A line's result as the column on the right draws it, already written in the language on screen. */
export interface ShownResult {
  readonly line: number;
  readonly text: string;
  readonly approx: boolean;
  /** Why the line has no result, said in `text`. */
  readonly failure: CalcError['kind'] | null;
  /** A line that names a variable: quieter than the lines it leads to. */
  readonly named: boolean;
  readonly selected: boolean;
  /** What the copy icon is called: « Copier 100,521 € ». */
  readonly copyLabel: string;
  readonly pickLabel: string;
}

const COPY_ICON =
  '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><rect x="9" y="9" width="11" height="11" rx="2"/><path d="M5 15V6a2 2 0 0 1 2-2h9"/></svg>';

/**
 * At the end of its line, laid over the right edge: the column of results stays aligned with
 * the lines it answers, however they scroll. Its clicks are the editor's handlers', by data
 * attribute, and never move the caret.
 */
class ResultWidget extends WidgetType {
  constructor(private readonly result: ShownResult) {
    super();
  }

  override eq(other: ResultWidget): boolean {
    const [a, b] = [this.result, other.result];
    return (
      a.line === b.line &&
      a.text === b.text &&
      a.approx === b.approx &&
      a.failure === b.failure &&
      a.named === b.named &&
      a.selected === b.selected &&
      a.copyLabel === b.copyLabel
    );
  }

  toDOM(): HTMLElement {
    const { result } = this;
    const box = document.createElement('span');
    box.className = 'calc-result';
    box.classList.toggle('calc-failed', result.failure !== null);
    if (result.failure) box.dataset['error'] = result.failure;
    box.classList.toggle('calc-named', result.named);
    box.classList.toggle('calc-selected', result.selected);
    box.dataset['testid'] = 'calc-result';
    box.dataset['line'] = String(result.line);
    if (result.failure === null) {
      const copy = document.createElement('button');
      copy.type = 'button';
      copy.className = 'calc-copy';
      copy.tabIndex = -1;
      copy.dataset['copy'] = String(result.line);
      copy.setAttribute('aria-label', result.copyLabel);
      copy.title = result.copyLabel;
      copy.innerHTML = COPY_ICON;
      box.append(copy);
    }
    const value = document.createElement('span');
    value.className = 'calc-value';
    value.title = result.failure ? result.text : result.pickLabel;
    if (result.failure === null) value.dataset['pick'] = String(result.line);
    value.textContent = `${result.approx ? '≈ ' : ''}${result.text}`;
    box.append(value);
    return box;
  }

  /** A press reaches the editor's handler, which picks or copies and keeps the caret where it was. */
  override ignoreEvent(event: Event): boolean {
    return event.type !== 'mousedown' && event.type !== 'click';
  }
}

export const setSheetMarks = StateEffect.define<DecorationSet>();

/** Mapped through each edit until the next answer replaces it: nothing jumps while typing. */
export const sheetMarks = StateField.define<DecorationSet>({
  create: () => Decoration.none,
  update(marks, transaction) {
    let next = marks.map(transaction.changes);
    for (const effect of transaction.effects) {
      if (effect.is(setSheetMarks)) next = effect.value;
    }
    return next;
  },
  provide: (field) => EditorView.decorations.from(field),
});

/** The colours of the words and the results, for a text the sheet was computed from. */
export function marksFor(doc: Text, sheet: CalcSheet, results: readonly ShownResult[]): DecorationSet {
  const builder = new RangeSetBuilder<Decoration>();
  const byLine = new Map(results.map((result) => [result.line, result]));
  sheet.rows.forEach((row, index) => {
    if (index >= doc.lines) return;
    const line = doc.line(index + 1);
    if (row.line.kind === 'heading') {
      builder.add(line.from, line.from, Decoration.line({ class: 'calc-heading' }));
    } else if (row.line.kind === 'value' && row.line.aggregate) {
      builder.add(line.from, line.from, Decoration.line({ class: 'calc-total' }));
    }
    for (const span of row.spans) {
      const from = Math.min(line.from + span.start, line.to);
      const to = Math.min(line.from + span.end, line.to);
      if (to > from) builder.add(from, to, Decoration.mark({ class: `calc-${span.kind}` }));
    }
    const result = byLine.get(index);
    if (result)
      builder.add(line.to, line.to, Decoration.widget({ widget: new ResultWidget(result), side: 1 }));
  });
  return builder.finish();
}
