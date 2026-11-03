import { Extension } from '@tiptap/core';
import type { Node as ProseMirrorNode } from '@tiptap/pm/model';
import { EditorState, Plugin, PluginKey, TextSelection, type Transaction } from '@tiptap/pm/state';
import { Decoration, DecorationSet, type EditorView } from '@tiptap/pm/view';
import { DiagramResult, MERMAID } from '@core/services/diagrams/mermaid-renderer';

export interface DiagramHooks {
  draw(source: string): Promise<DiagramResult>;
  /** What hovering a drawing says: a click on it goes to its code. */
  title(): string;
  failed(line: number | null): string;
}

/** The label a diagram's block carries, where a code block names its language. */
export const MERMAID_LABEL = 'MERMAID';

/** Typing redraws once the keys pause; the drawing before stays on screen meanwhile. */
export const REDRAW_AFTER_MS = 300;

interface DiagramsState {
  readonly focused: boolean;
  readonly decorations: DecorationSet;
}

type WidgetFor = (
  source: string,
  rank: number,
) => (view: EditorView, getPos: () => number | undefined) => Node;

const diagramsKey = new PluginKey<DiagramsState>('diagrams');

export function isDiagram(node: ProseMirrorNode): boolean {
  return node.type.name === 'codeBlock' && node.attrs['language'] === MERMAID;
}

/**
 * A `mermaid` code block drawn under its code. Away from the caret the code folds to its first
 * line and the drawing is what the Note shows; a click on the drawing goes back to the code.
 */
export const Diagrams = (hooks: DiagramHooks): Extension => {
  /** The last drawing of each diagram, by its rank in the Note. */
  const shown = new Map<number, DiagramResult>();

  const widgetFor: WidgetFor = (source, rank) => (view, getPos) => {
    const host = document.createElement('div');
    host.className = 'rich-diagram';
    host.contentEditable = 'false';
    host.dataset['testid'] = 'rich-diagram';
    host.title = hooks.title();
    const previous = shown.get(rank);
    if (previous) paint(host, previous, hooks);

    host.addEventListener('mousedown', (event) => {
      event.preventDefault();
      const end = getPos();
      if (end === undefined) return;
      // ⚠️ Focused before the selection is set: see `focusFirst`.
      view.focus();
      view.dispatch(view.state.tr.setSelection(TextSelection.create(view.state.doc, end - 1)));
    });

    setTimeout(
      () => {
        if (!host.isConnected) return;
        void hooks.draw(source).then((result) => {
          shown.set(rank, result);
          if (host.isConnected) paint(host, result, hooks);
        });
      },
      previous ? REDRAW_AFTER_MS : 0,
    );
    return host;
  };

  return Extension.create({
    name: 'diagrams',
    addProseMirrorPlugins() {
      return [
        new Plugin<DiagramsState>({
          key: diagramsKey,
          state: {
            init: (_, state) => ({ focused: false, decorations: decorate(state, false, widgetFor) }),
            apply: (tr, previous, _before, state) => {
              const focused = focusAfter(tr, previous.focused);
              const changed =
                tr.docChanged ||
                tr.selectionSet ||
                focused !== previous.focused ||
                tr.getMeta(diagramsKey) === true;
              return changed ? { focused, decorations: decorate(state, focused, widgetFor) } : previous;
            },
          },
          props: {
            decorations(state) {
              return this.getState(state)?.decorations;
            },
          },
          // A theme changed in the settings: every drawing is drawn again in its colours.
          view: (view) => {
            const observer = new MutationObserver(() =>
              view.dispatch(view.state.tr.setMeta(diagramsKey, true)),
            );
            observer.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] });
            return { destroy: () => observer.disconnect() };
          },
        }),
      ];
    },
  });
};

/** TipTap marks the transactions focus and blur dispatch. */
function focusAfter(tr: Transaction, focused: boolean): boolean {
  if (tr.getMeta('focus') !== undefined) return true;
  if (tr.getMeta('blur') !== undefined) return false;
  return focused;
}

function decorate(state: EditorState, focused: boolean, widgetFor: WidgetFor): DecorationSet {
  const decorations: Decoration[] = [];
  const { from, to } = state.selection;
  const theme = document.documentElement.dataset['theme'] ?? 'dark';
  let rank = 0;

  state.doc.descendants((node, position) => {
    if (node.type.name !== 'codeBlock') return true;
    if (!isDiagram(node)) return false;

    const source = node.textContent;
    const at = rank++;
    if (source.trim() === '') return false;

    const end = position + node.nodeSize;
    if (!(focused && from >= position && to <= end)) {
      decorations.push(Decoration.node(position, end, { class: 'is-folded' }));
    }
    decorations.push(
      Decoration.widget(end, widgetFor(source, at), {
        key: `diagram-${at}-${theme}-${hashOf(source)}`,
        side: -1,
        ignoreSelection: true,
        stopEvent: () => true,
      }),
    );
    return false;
  });

  return DecorationSet.create(state.doc, decorations);
}

/** The SVG is read by the HTML parser, as it was written; an inert document runs nothing. */
function paint(host: HTMLElement, result: DiagramResult, hooks: DiagramHooks): void {
  if ('svg' in result) {
    const svg = new DOMParser().parseFromString(result.svg, 'text/html').body.firstElementChild;
    host.classList.remove('is-failed');
    host.replaceChildren(...(svg ? [document.importNode(svg, true)] : []));
    return;
  }

  const sentence = document.createElement('p');
  sentence.className = 'rich-diagram-error';
  sentence.textContent = hooks.failed(result.line);
  const detail = document.createElement('div');
  detail.className = 'rich-diagram-detail';
  detail.textContent = result.error;
  host.classList.add('is-failed');
  host.replaceChildren(sentence, detail);
}

function hashOf(text: string): string {
  let hash = 5381;
  for (let index = 0; index < text.length; index++) {
    hash = (hash * 33) ^ (text.codePointAt(index) ?? 0);
  }
  return `${text.length}.${(hash >>> 0).toString(36)}`;
}
