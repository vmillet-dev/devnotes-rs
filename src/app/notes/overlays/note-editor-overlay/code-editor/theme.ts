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
  '&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground, .cm-content ::selection': {
    backgroundColor: 'rgba(var(--amber-rgb), 0.3)',
  },
  // Unfocused, a selection is the visualiser's, already lit in blue: a veil, not a second band.
  '.cm-selectionBackground': { backgroundColor: 'rgba(var(--amber-rgb), 0.1)' },
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
  // A `{{field}}` where it is written, in the fields panel's amber, its default dimmed: the
  // language's colours give way inside, whichever of the two marks is drawn outside.
  '.cm-field': {
    color: 'var(--amber-text)',
    backgroundColor: 'rgba(var(--amber-rgb), 0.1)',
    borderRadius: '3px',
  },
  '.cm-field span': { color: 'inherit' },
  '.cm-field .cm-field-default': { color: 'var(--text-2)' },
  '.cm-field .cm-field-default span': { color: 'inherit' },
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
  // Go to line (Ctrl+Alt+G) is CodeMirror's dialog, drawn with the search bar's field and buttons.
  '.cm-dialog': {
    display: 'flex',
    alignItems: 'center',
    gap: '6px',
    padding: '6px 12px 6px 38px',
    fontFamily: 'var(--font-sans), sans-serif',
    fontSize: '12px',
  },
  '.cm-dialog form': { display: 'flex', alignItems: 'center', gap: '6px' },
  '.cm-dialog label': { display: 'flex', alignItems: 'center', gap: '8px', color: 'var(--text-1)' },
  '.cm-textfield': {
    width: '200px',
    height: '30px',
    boxSizing: 'border-box',
    padding: '0 9px',
    border: '1px solid var(--line)',
    borderRadius: '7px',
    backgroundColor: 'var(--bg-1)',
    color: 'var(--text-0)',
    fontFamily: 'var(--font-mono), monospace',
    fontSize: '12.5px',
  },
  '.cm-textfield:focus-visible': { outline: '2px solid var(--amber-edge)', outlineOffset: '-2px' },
  '.cm-button': {
    margin: '0',
    padding: '4px 10px',
    border: '1px solid var(--line-soft)',
    borderRadius: '7px',
    backgroundImage: 'none',
    backgroundColor: 'var(--bg-1)',
    color: 'var(--text-1)',
    fontFamily: 'inherit',
    fontSize: '11.5px',
    cursor: 'pointer',
  },
  '.cm-button:hover': { borderColor: 'var(--amber-dim)', color: 'var(--text-0)' },
  '.cm-dialog .cm-dialog-close': {
    position: 'static',
    marginLeft: 'auto',
    width: '24px',
    height: '24px',
    padding: '0',
    border: 'none',
    borderRadius: '6px',
    background: 'none',
    color: 'var(--text-1)',
    fontSize: '18px',
    lineHeight: '1',
    cursor: 'pointer',
  },
  '.cm-dialog .cm-dialog-close:hover': { backgroundColor: 'var(--bg-3)', color: 'var(--text-0)' },
  '.cm-button:focus-visible, .cm-dialog-close:focus-visible': {
    outline: '2px solid var(--amber-edge)',
    outlineOffset: '-2px',
  },
});
