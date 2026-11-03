import type { StreamParser, StringStream } from '@codemirror/language';

/** What a language of ours is coloured by: words, comments, strings and numbers, nothing parsed. */
export interface SmallModeSpec {
  readonly name: string;
  readonly keywords: readonly string[];
  readonly types?: readonly string[];
  /** A word that is a type by its shape — `u8`, `uint256` — rather than by a list. */
  readonly typePattern?: RegExp;
  readonly atoms?: readonly string[];
  readonly lineComments: readonly string[];
  readonly blockComment?: readonly [open: string, close: string];
  /** Each opens and closes a string; a backslash escapes inside them. */
  readonly quotes: readonly string[];
  /** A word written with a sigil — `@import`, `@id` — coloured as an annotation. */
  readonly annotation?: RegExp;
}

interface SmallModeState {
  inComment: boolean;
  quote: string | null;
}

const NUMBER = /^(?:0x[\da-f_]+|0b[01_]+|0o[0-7_]+|\d[\d_]*(?:\.\d[\d_]*)?(?:e[+-]?\d+)?)/i;
const WORD = /^[A-Za-z_][\w-]*/;

export function smallMode(spec: SmallModeSpec): StreamParser<SmallModeState> {
  const keywords = new Set(spec.keywords);
  const types = new Set(spec.types);
  const atoms = new Set(spec.atoms);

  function inQuote(stream: StringStream, state: SmallModeState, quote: string): string {
    while (!stream.eol()) {
      if (stream.match(quote)) {
        state.quote = null;
        return 'string';
      }
      if (stream.next() === '\\') stream.next();
    }
    return 'string';
  }

  function inComment(stream: StringStream, state: SmallModeState, close: string): string {
    if (stream.skipTo(close)) {
      stream.match(close);
      state.inComment = false;
    } else {
      stream.skipToEnd();
    }
    return 'comment';
  }

  return {
    name: spec.name,
    startState: () => ({ inComment: false, quote: null }),
    copyState: (state) => ({ ...state }),
    token(stream, state) {
      if (state.inComment && spec.blockComment) return inComment(stream, state, spec.blockComment[1]);
      if (state.quote) return inQuote(stream, state, state.quote);
      if (stream.eatSpace()) return null;

      if (spec.lineComments.some((marker) => stream.match(marker))) {
        stream.skipToEnd();
        return 'comment';
      }
      if (spec.blockComment && stream.match(spec.blockComment[0])) {
        state.inComment = true;
        return inComment(stream, state, spec.blockComment[1]);
      }
      const quote = spec.quotes.find((opening) => stream.match(opening));
      if (quote) {
        state.quote = quote;
        return inQuote(stream, state, quote);
      }
      if (spec.annotation && stream.match(spec.annotation)) return 'meta';
      if (stream.match(NUMBER)) return 'number';

      const word = stream.match(WORD);
      if (word && typeof word !== 'boolean') {
        const text = word[0];
        if (keywords.has(text)) return 'keyword';
        if (atoms.has(text)) return 'atom';
        if (types.has(text) || spec.typePattern?.test(text)) return 'type';
        return stream.peek() === '(' ? 'variableName.function' : 'variable';
      }

      stream.next();
      return null;
    },
    languageData: {
      commentTokens: {
        ...(spec.lineComments[0] && { line: spec.lineComments[0] }),
        ...(spec.blockComment && { block: { open: spec.blockComment[0], close: spec.blockComment[1] } }),
      },
    },
  };
}
