import type { Language } from '@codemirror/language';
import { highlightCode, highlightTree, tagHighlighter, tags as t } from '@lezer/highlight';

/**
 * Lezer's tags as the classes `src/styles/_code-theme.scss` colours, for the editor and the
 * cards alike. The most specific tag wins, so `null` and `atom` — keywords to Lezer — come
 * back to the literals' colour.
 */
export const CODE_HIGHLIGHTER = tagHighlighter([
  {
    tag: [t.keyword, t.typeName, t.standard(t.variableName), t.operator, t.angleBracket],
    class: 'tok-keyword',
  },
  { tag: [t.string, t.regexp, t.escape, t.link, t.url, t.quote, t.inserted], class: 'tok-string' },
  {
    tag: [t.number, t.bool, t.null, t.atom, t.color, t.unit, t.constant(t.variableName)],
    class: 'tok-literal',
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
    class: 'tok-name',
  },
  { tag: [t.propertyName, t.attributeName, t.variableName], class: 'tok-property' },
  { tag: [t.meta, t.annotation, t.processingInstruction], class: 'tok-meta' },
  { tag: t.comment, class: 'tok-comment' },
  { tag: t.deleted, class: 'tok-deleted' },
  { tag: t.emphasis, class: 'tok-emphasis' },
  { tag: t.strong, class: 'tok-strong' },
]);

const HTML_ESCAPES: Readonly<Record<string, string>> = {
  '&': '&amp;',
  '<': '&lt;',
  '>': '&gt;',
};

function escapeHtml(text: string): string {
  return text.replace(/[&<>]/g, (character) => HTML_ESCAPES[character] ?? character);
}

/**
 * One line of HTML per line of the text, each holding only `<span class="tok-…">` around text
 * escaped here: it passes Angular's sanitizer intact, and must never be marked as safe — a
 * note's content is typed by the user. Without a grammar — plain text, or one still loading —
 * the text is only escaped.
 */
export function highlightLines(content: string, grammar: Language | null): string[] {
  if (!grammar) return content.split('\n').map(escapeHtml);

  const lines: string[] = [];
  let current = '';
  highlightCode(
    content,
    grammar.parser.parse(content),
    CODE_HIGHLIGHTER,
    (text, classes) => {
      current += classes ? `<span class="${classes}">${escapeHtml(text)}</span>` : escapeHtml(text);
    },
    () => {
      lines.push(current);
      current = '';
    },
  );
  lines.push(current);
  return lines;
}

/** One coloured stretch of a text, by offsets into it rather than as HTML. */
export interface HighlightRange {
  readonly from: number;
  readonly to: number;
  readonly classes: string;
}

/** For an editor that holds the text itself and colours it with decorations. */
export function highlightRanges(content: string, grammar: Language | null): HighlightRange[] {
  if (!grammar) return [];

  const ranges: HighlightRange[] = [];
  highlightTree(grammar.parser.parse(content), CODE_HIGHLIGHTER, (from, to, classes) => {
    ranges.push({ from, to, classes });
  });
  return ranges;
}
