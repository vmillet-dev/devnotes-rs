import { LineEnding } from '@core/model/tool-answers.model';

export interface VisibleLine {
  readonly content: string;
  /** Spaces and tabs at the end, drawn apart so a trim can be seen. */
  readonly trailing: string;
  readonly ending: LineEnding | null;
}

export interface VisibleText {
  readonly lines: readonly VisibleLine[];
  readonly hidden: number;
}

const ENDINGS: Record<string, LineEnding> = { '\n': 'lf', '\r\n': 'crlf', '\r': 'cr' };

/** Enough to see what a text is made of; the rest is counted, not drawn. */
export const MAX_VISIBLE_LINES = 400;

/** Drawing only: which endings a text has, and what they become, is Rust's to say. */
export function visibleLines(text: string): VisibleText {
  const pieces = text.split(/(\r\n|\n|\r)/);
  const lines: VisibleLine[] = [];
  for (let index = 0; index < pieces.length; index += 2) {
    const line = pieces[index]!;
    const content = line.replace(/[ \t]+$/, '');
    lines.push({
      content,
      trailing: line.slice(content.length),
      ending: ENDINGS[pieces[index + 1] ?? ''] ?? null,
    });
  }
  // A text ending with a newline ends on an empty line with nothing after it: not a line to draw.
  const last = lines.at(-1);
  if (lines.length > 1 && last?.content === '' && last.trailing === '' && last.ending === null) {
    lines.pop();
  }
  return { lines: lines.slice(0, MAX_VISIBLE_LINES), hidden: Math.max(0, lines.length - MAX_VISIBLE_LINES) };
}
