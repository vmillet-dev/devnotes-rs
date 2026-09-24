const FIELD = /\{\{[^{}\n]*\}\}/g;
const PUNCTUATION = /[!-/:-@[-`{-~]/;
const WORD = /[\p{L}\p{N}]/u;
const SPACE = /\s/;

/**
 * A text run written back as Markdown, escaped only where a character would read as syntax.
 *
 * ⚠️ Replaces TipTap's own encoding, which escapes every `_ * ~ [ ] \` and turns `& < >` into
 * entities: `{{db_host}}` stopped being a field, and "a -> b" was stored as `a -&gt; b`, then
 * copied and searched that way. A `{{field}}` is left exactly as typed.
 */
export function escapeMarkdownText(text: string): string {
  let out = '';
  let from = 0;
  for (const field of text.matchAll(FIELD)) {
    out += escapeRun(text.slice(from, field.index));
    out += field[0];
    from = field.index + field[0].length;
  }

  return out + escapeRun(text.slice(from));
}

function escapeRun(run: string): string {
  let out = '';
  for (let at = 0; at < run.length; at += 1) {
    const char = run[at]!;
    out += needsEscape(char, run[at - 1], run[at + 1], run.slice(at)) ? `\\${char}` : char;
  }

  return out;
}

function needsEscape(
  char: string,
  before: string | undefined,
  after: string | undefined,
  rest: string,
): boolean {
  const spaceBefore = before === undefined || SPACE.test(before);
  const spaceAfter = after === undefined || SPACE.test(after);

  switch (char) {
    case '\\':
      return after !== undefined && PUNCTUATION.test(after);
    case '`':
    case '[':
      return true;
    // Emphasis needs a flank: `2 * 3` and `snake_case` are left as they are.
    case '*':
      return !(spaceBefore && spaceAfter);
    case '_':
      return !(spaceBefore && spaceAfter) && !(isWord(before) && isWord(after));
    // `~/.ssh` is a path; `~~` and `~word` could strike through.
    case '~':
      return !spaceAfter && after !== '/';
    case '<':
      return after !== undefined && /[A-Za-z/!?]/.test(after);
    case '&':
      return /^&(#\d+|#x[0-9a-f]+|[a-z][a-z0-9]*);/i.test(rest);
    default:
      return false;
  }
}

function isWord(char: string | undefined): boolean {
  return char !== undefined && WORD.test(char);
}
