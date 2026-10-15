import { RangeSetBuilder, countColumn } from '@codemirror/state';
import { Decoration, DecorationSet, EditorView, ViewPlugin, ViewUpdate } from '@codemirror/view';

/** How much further in than the line's own text a wrapped row starts. */
export const HANG_COLUMNS = 2;

/** Columns of leading whitespace, a tab reaching the next multiple of the tab size. */
export function leadingColumns(text: string, tabSize: number): number {
  const indent = /^[ \t]*/.exec(text)![0];
  return countColumn(indent, tabSize);
}

const byColumns = new Map<number, Decoration>();

function hangOf(columns: number): Decoration {
  let decoration = byColumns.get(columns);
  if (!decoration) {
    decoration = Decoration.line({ class: 'cm-hang', attributes: { style: `--hang: ${columns}ch` } });
    byColumns.set(columns, decoration);
  }
  return decoration;
}

function decorate(view: EditorView): DecorationSet {
  const builder = new RangeSetBuilder<Decoration>();
  const { doc, tabSize } = view.state;
  let last = 0;
  for (const { from, to } of view.visibleRanges) {
    for (let at = from; at <= to;) {
      const line = doc.lineAt(at);
      if (line.number > last && line.length > 0) {
        builder.add(line.from, line.from, hangOf(leadingColumns(line.text, tabSize) + HANG_COLUMNS));
      }
      last = Math.max(last, line.number);
      at = line.to + 1;
    }
  }
  return builder.finish();
}

class HangingIndent {
  decorations: DecorationSet;

  constructor(private readonly view: EditorView) {
    this.decorations = decorate(view);
    this.measureCap();
  }

  update(update: ViewUpdate): void {
    if (update.docChanged || update.viewportChanged || update.startState.tabSize !== update.state.tabSize) {
      this.decorations = decorate(update.view);
    }
    if (update.geometryChanged) this.measureCap();
  }

  destroy(): void {
    this.view.dom.style.removeProperty('--hang-cap');
  }

  /**
   * Past half the text's width a deep indentation would leave a column of a few characters. In
   * pixels: `padding-left` and `text-indent` read a percentage against two different widths.
   */
  private measureCap(): void {
    this.view.requestMeasure({
      key: this,
      read: (view) => view.contentDOM.clientWidth,
      write: (width, view) => {
        if (width > 0) view.dom.style.setProperty('--hang-cap', `${Math.floor(width / 2)}px`);
      },
    });
  }
}

/**
 * A wrapped row continues under the line's text, two columns further in; the first row does not
 * move. Visible lines only, so a long document pays for what is on screen.
 */
export const hangingIndent = [
  ViewPlugin.fromClass(HangingIndent, { decorations: (plugin) => plugin.decorations }),
  EditorView.theme({
    '.cm-line.cm-hang': {
      paddingLeft: 'min(var(--hang), var(--hang-cap, var(--hang)))',
      textIndent: 'calc(-1 * min(var(--hang), var(--hang-cap, var(--hang))))',
    },
  }),
];
