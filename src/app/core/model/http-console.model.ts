import type { ConsoleLine, ConsoleOrigin } from '@core/ipc/bindings';

export type { ConsoleEvent, ConsoleLine, ConsoleOrigin } from '@core/ipc/bindings';

/** A line of a tab's console: when it was said, and the send it belongs to. */
export interface ConsoleEntry {
  readonly at: number;
  readonly sendId: string;
  /** The request sent, and in a run of several its iteration, from 1. */
  readonly request: string;
  readonly iteration: number | null;
  readonly line: ConsoleLine;
}

/** A run's sends are named `run:<id>#…`: its console is one, under `run:<id>`. */
export function runConsoleKey(sendId: string): string | null {
  return sendId.startsWith('run:')
    ? sendId.slice(0, sendId.indexOf('#') < 0 ? undefined : sendId.indexOf('#'))
    : null;
}

export type ConsoleFilter = 'all' | 'traffic' | 'logs' | 'errors';
export const CONSOLE_FILTERS: readonly ConsoleFilter[] = ['all', 'traffic', 'logs', 'errors'];

/** What a tab keeps, the oldest dropped first: the session's, never the disk's. */
export const CONSOLE_LINES_KEPT = 500;

/** Which of the three filters a line falls under. */
export function consoleKindOf(line: ConsoleLine): Exclude<ConsoleFilter, 'all'> {
  switch (line.kind) {
    case 'out':
    case 'in':
      return 'traffic';
    case 'log':
      return 'logs';
    case 'failed':
    case 'test':
    case 'stopped':
    case 'missed':
      return 'errors';
  }
}

export function consoleCounts(entries: readonly ConsoleEntry[]): Readonly<Record<ConsoleFilter, number>> {
  const counts = { all: entries.length, traffic: 0, logs: 0, errors: 0 };
  for (const entry of entries) counts[consoleKindOf(entry.line)]++;
  return counts;
}

/** A text cut where a secret is written as its `{{name}}`: the console shows those apart. */
export function secretSegments(text: string): readonly { readonly text: string; readonly secret: boolean }[] {
  return text
    .split(/(\{\{[^{}]+\}\})/)
    .filter((part) => part !== '')
    .map((part) => ({ text: part, secret: /^\{\{[^{}]+\}\}$/.test(part) }));
}

/** `14:02:11.998`, the local time a line was said. */
export function consoleTime(at: number): string {
  const date = new Date(at);
  const two = (value: number) => String(value).padStart(2, '0');
  return `${two(date.getHours())}:${two(date.getMinutes())}:${two(date.getSeconds())}.${String(date.getMilliseconds()).padStart(3, '0')}`;
}

/** Who said a line: the script's kind and holder; a send of a script says so. */
export function originKey(line: ConsoleLine, by: ConsoleOrigin): string {
  return line.kind === 'out' || line.kind === 'in' || line.kind === 'failed'
    ? 'http.console.origins.sent'
    : `http.console.origins.${by.kind}`;
}
