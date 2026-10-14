import { HighlightStyle } from '@codemirror/language';
import { EditorView } from '@codemirror/view';
import { tags as t } from '@lezer/highlight';

/**
 * The colours of `src/styles/_code-theme.scss`, tag for tag: the card and the fill preview are
 * highlight.js, the editor is Lezer, and both draw one palette. The most specific tag wins, so
 * `null` and `atom` — keywords to Lezer — come back to the literals' colour.
 */
export const CODE_COLOURS = HighlightStyle.define([
  {
    tag: [t.keyword, t.typeName, t.standard(t.variableName), t.operator, t.angleBracket],
    color: 'var(--blue)',
  },
  {
    tag: [t.string, t.regexp, t.escape, t.link, t.url, t.quote, t.inserted],
    color: 'var(--green)',
  },
  {
    tag: [t.number, t.bool, t.null, t.atom, t.color, t.unit, t.constant(t.variableName)],
    color: 'var(--amber-text)',
  },
  {
    tag: [
      t.function(t.variableName),
      t.function(t.propertyName),
      t.function(t.definition(t.variableName)),
      t.className,
      t.tagName,
      t.labelName,
      t.macroName,
      t.heading,
    ],
    color: 'var(--purple)',
  },
  { tag: [t.propertyName, t.attributeName, t.variableName], color: 'var(--text-0)' },
  { tag: [t.meta, t.annotation, t.processingInstruction], color: 'var(--text-1)' },
  { tag: t.comment, color: 'var(--text-2)', fontStyle: 'italic' },
  { tag: t.deleted, color: 'var(--red)' },
  { tag: t.emphasis, fontStyle: 'italic' },
  { tag: t.strong, fontWeight: '700' },
]);

/**
 * The code field's metrics, as the `<textarea>` over the viewer had them: 13px at 1.7, the
 * text 56px in — the padding and a 32px gutter — and the selection in amber.
 */
export const CODE_THEME = EditorView.theme({
  '&': { height: '100%', fontSize: '13px', color: 'var(--text-0)', backgroundColor: 'transparent' },
  '&.cm-focused': { outline: '2px solid var(--amber-edge)', outlineOffset: '-2px' },
  '.cm-scroller': { fontFamily: 'var(--font-mono), monospace', lineHeight: '1.7' },
  '.cm-content': { padding: '20px 24px 20px 0', caretColor: 'var(--text-0)' },
  '.cm-line': { padding: '0' },
  '.cm-cursor, .cm-dropCursor': { borderLeftColor: 'var(--text-0)' },
  '&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection':
    { backgroundColor: 'rgba(var(--amber-rgb), 0.3)' },
  '.cm-gutters': {
    paddingLeft: '24px',
    border: 'none',
    backgroundColor: 'transparent',
    color: 'var(--line-no)',
  },
  '.cm-lineNumbers .cm-gutterElement': { minWidth: '32px', padding: '0 16px 0 0', boxSizing: 'border-box' },
  '.cm-placeholder': { color: 'var(--text-2)', fontStyle: 'italic' },
  '&.cm-focused .cm-matchingBracket': { backgroundColor: 'rgba(var(--blue-rgb), 0.2)' },
  '&.cm-focused .cm-nonmatchingBracket': { backgroundColor: 'rgba(var(--red-rgb), 0.2)' },
  '.cm-searchMatch': { backgroundColor: 'rgba(var(--amber-rgb), 0.18)' },
  '.cm-searchMatch.cm-searchMatch-selected': { backgroundColor: 'rgba(var(--amber-rgb), 0.4)' },
  '.cm-selectionMatch': { backgroundColor: 'rgba(var(--text-1-rgb), 0.12)' },
  // What the format just changed, and what the visualiser beside the code has selected.
  '.cm-line.cm-marked': {
    backgroundColor: 'rgba(var(--amber-rgb), 0.07)',
    boxShadow: 'inset 3px 0 var(--amber-edge)',
  },
  '.cm-line.cm-focused-line': {
    backgroundColor: 'rgba(var(--blue-rgb), 0.1)',
    boxShadow: 'inset 3px 0 var(--blue)',
  },
  '.cm-panels': { backgroundColor: 'var(--bg-2)', color: 'var(--text-1)' },
  '.cm-panels.cm-panels-top': { borderBottom: '1px solid var(--line)' },
  '.cm-panels.cm-panels-bottom': { borderTop: '1px solid var(--line)' },
  '.cm-panel.cm-search, .cm-panel.cm-gotoLine': { padding: '8px 12px', fontSize: '12.5px' },
  // A button and a field take no font from their parent unless told to.
  '.cm-panel button, .cm-panel input': { fontFamily: 'inherit' },
  '.cm-textfield': {
    padding: '4px 8px',
    border: '1px solid var(--line)',
    borderRadius: '6px',
    backgroundColor: 'var(--bg-1)',
    color: 'var(--text-0)',
    fontSize: '12.5px',
  },
  '.cm-textfield:focus-visible': { outline: '2px solid var(--amber-edge)', outlineOffset: '-2px' },
  '.cm-button': {
    padding: '4px 10px',
    border: '1px solid var(--line)',
    borderRadius: '6px',
    backgroundImage: 'none',
    backgroundColor: 'var(--bg-3)',
    color: 'var(--text-0)',
    fontSize: '12px',
    cursor: 'pointer',
  },
  '.cm-button:focus-visible, .cm-panel button[name=close]:focus-visible': {
    outline: '2px solid var(--amber-edge)',
    outlineOffset: '1px',
  },
  '.cm-panel label': { fontSize: '12px', color: 'var(--text-1)' },
  '.cm-panel button[name=close]': { color: 'var(--text-1)', cursor: 'pointer' },
});
