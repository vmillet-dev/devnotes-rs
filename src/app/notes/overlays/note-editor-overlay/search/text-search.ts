import { SearchSpec } from './search-target';

export interface TextMatch {
  /** Code units, as `String` and ProseMirror count them. */
  readonly from: number;
  readonly to: number;
  readonly text: string;
}

/** A letter, a digit or `_` on either side and the match is inside a word, as CodeMirror decides. */
const WORD = String.raw`[\p{L}\p{N}_]`;

/** The query as a pattern; `null` for an empty one, or a regular expression that does not compile. */
export function searchPattern(spec: SearchSpec): RegExp | null {
  if (spec.search === '') return null;
  const body = spec.regexp ? spec.search : spec.search.replace(/[.*+?^${}()|[\]\\]/g, String.raw`\$&`);
  const source = spec.wholeWord ? `(?<!${WORD})(?:${body})(?!${WORD})` : body;
  try {
    return new RegExp(source, spec.caseSensitive ? 'gu' : 'giu');
  } catch {
    return null;
  }
}

/** Every match in `text`, an empty one skipped: `^` or `a*` would otherwise find nothing forever. */
export function findMatches(text: string, pattern: RegExp): TextMatch[] {
  const found: TextMatch[] = [];
  pattern.lastIndex = 0;
  for (let match = pattern.exec(text); match !== null; match = pattern.exec(text)) {
    if (match[0] === '') {
      pattern.lastIndex += 1;
      continue;
    }
    found.push({ from: match.index, to: match.index + match[0].length, text: match[0] });
  }
  return found;
}

/** What replaces one match: the text as typed, or with a regular expression its `$1` and `$&` read. */
export function replacementOf(match: TextMatch, spec: SearchSpec, pattern: RegExp): string {
  if (!spec.regexp) return spec.replace;
  return match.text.replace(new RegExp(pattern.source, pattern.flags.replace('g', '')), spec.replace);
}
