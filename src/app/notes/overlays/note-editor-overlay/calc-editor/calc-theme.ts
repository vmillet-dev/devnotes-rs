import { EditorView } from '@codemirror/view';

/** Room on the right of each line for its result: the column `ResultWidget` is laid in. */
const RESULTS_WIDTH = '260px';

/**
 * A sheet's colours and its column of results, scoped by CodeMirror to this editor alone:
 * the words in the hues the mockup gives them, a rule above a total, the message of a line
 * in red where its result would stand.
 */
export const CALC_THEME = EditorView.theme({
  '.cm-content': { paddingRight: '0' },
  '.cm-line': { position: 'relative', paddingRight: `calc(${RESULTS_WIDTH} + 16px)` },
  '.calc-number': { color: 'var(--amber-text)' },
  '.calc-unit': { color: 'var(--teal)' },
  '.calc-operator': { color: 'var(--text-1)' },
  '.calc-keyword': { color: 'var(--purple)' },
  '.calc-name': { color: 'var(--blue)' },
  '.calc-date': { color: 'var(--green)' },
  '.calc-comment': { color: 'var(--text-2)', fontStyle: 'italic' },
  '.calc-heading': { fontWeight: '700', color: 'var(--text-0)' },
  '.calc-error': { textDecoration: 'underline wavy var(--red)', textUnderlineOffset: '3px' },
  '.calc-result': {
    position: 'absolute',
    top: '0',
    right: '8px',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'flex-end',
    gap: '6px',
    maxWidth: RESULTS_WIDTH,
    padding: '0 8px',
    borderRadius: '5px',
    fontVariantNumeric: 'tabular-nums',
    color: 'var(--text-0)',
    whiteSpace: 'nowrap',
    userSelect: 'none',
    textDecoration: 'none',
  },
  '.calc-total .calc-result': { borderTop: '1px solid var(--text-1)', borderRadius: '0' },
  '.calc-result.calc-named': { color: 'var(--text-1)' },
  '.calc-result.calc-selected': { backgroundColor: 'rgba(var(--pink-rgb), 0.16)', color: 'var(--text-0)' },
  '.calc-result.calc-failed': {
    color: 'var(--red)',
    fontFamily: 'var(--font-sans), sans-serif',
    fontSize: '12px',
  },
  '.calc-value': { overflow: 'hidden', textOverflow: 'ellipsis', cursor: 'pointer' },
  '.calc-failed .calc-value': { cursor: 'default' },
  '.calc-copy': {
    display: 'inline-grid',
    placeItems: 'center',
    width: '22px',
    height: '22px',
    padding: '0',
    border: 'none',
    borderRadius: '5px',
    background: 'none',
    color: 'var(--text-2)',
    cursor: 'pointer',
    opacity: '0',
  },
  '.calc-copy svg': { width: '13px', height: '13px' },
  '.calc-copy:hover': { backgroundColor: 'var(--bg-3)', color: 'var(--amber-text)' },
  '.cm-line:hover .calc-copy, .calc-selected .calc-copy': { opacity: '1' },
  '.cm-tooltip': {
    border: '1px solid var(--line)',
    borderRadius: '7px',
    backgroundColor: 'var(--bg-2)',
    boxShadow: 'var(--shadow)',
  },
  '.calc-tooltip': {
    display: 'flex',
    flexDirection: 'column',
    gap: '2px',
    padding: '6px 10px',
    fontFamily: 'var(--font-sans), sans-serif',
    fontSize: '12px',
    color: 'var(--text-1)',
  },
  '.calc-tooltip strong': {
    fontFamily: 'var(--font-mono), monospace',
    fontWeight: '600',
    color: 'var(--text-0)',
  },
  '.cm-tooltip-autocomplete > ul > li': { padding: '2px 10px' },
  '.cm-tooltip-autocomplete > ul > li[aria-selected]': {
    backgroundColor: 'rgba(var(--amber-rgb), 0.2)',
    color: 'var(--text-0)',
  },
  '.cm-completionDetail': {
    marginLeft: '10px',
    fontFamily: 'var(--font-sans), sans-serif',
    fontStyle: 'normal',
    color: 'var(--text-2)',
  },
});
