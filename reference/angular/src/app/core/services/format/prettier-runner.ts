import type { Plugin } from 'prettier';
import { formatWithCursor } from 'prettier/standalone';
import { shieldFields, unshieldFields } from './field-shield';
import { FormatAnswer, FormatRequest, PrettierParser } from './format.model';
import { changedLines } from './line-diff';

export type PluginLoader = (parser: PrettierParser) => Promise<Plugin[]>;

/** What the worker runs: kept apart from it so a unit test drives the real Prettier. */
export async function runPrettier(request: FormatRequest, load: PluginLoader): Promise<FormatAnswer> {
  const shielded = shieldFields(request.text, request.cursor);
  if (shielded === null) return { kind: 'failed' };

  let result: { formatted: string; cursorOffset: number };
  try {
    result = await formatWithCursor(shielded.text, {
      ...request.options,
      parser: request.parser,
      plugins: await load(request.parser),
      cursorOffset: shielded.cursor,
      // Prettier's default is `lf`, which would rewrite a CRLF snippet's every line.
      endOfLine: 'auto',
    });
  } catch (error) {
    return syntaxError(error) ?? { kind: 'failed' };
  }

  const restored = unshieldFields(result.formatted, result.cursorOffset, shielded);
  if (restored === null) return { kind: 'fields' };

  // A snippet ends where it was written to end; Prettier's closing newline is a file's.
  const text = request.text.endsWith('\n') ? restored.text : restored.text.replace(/\r?\n$/, '');
  if (text === request.text) return { kind: 'unchanged' };

  return {
    kind: 'formatted',
    text,
    cursor: Math.min(restored.cursor, text.length),
    changedLines: changedLines(request.text, text),
  };
}

function syntaxError(error: unknown): FormatAnswer | null {
  const start = (error as { loc?: { start?: { line?: unknown; column?: unknown } } } | null)?.loc?.start;
  return typeof start?.line === 'number' && typeof start.column === 'number'
    ? { kind: 'syntax', line: start.line, column: start.column }
    : null;
}
