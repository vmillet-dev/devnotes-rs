import { ReferenceRow, ReferenceSource, matchesSearchable } from '@core/services/tools/reference.model';
import { SIGNALS, SignalWords, UnixSignal } from './signals.data';

export interface SignalRow extends ReferenceRow {
  readonly signal: UnixSignal;
  readonly use: string;
  /** macOS's number, Linux's when the signal does not differ; `null` where macOS has none. */
  readonly macos: string | null;
}

const DIGITS = /^\d+$/;

/** `34–64`, the real-time signals, holds every number between. */
function holds(numbers: string | null, wanted: number): boolean {
  if (numbers === null) return false;
  const [from = Number.NaN, to = from] = numbers.split('–').map(Number);
  return wanted >= from && wanted <= to;
}

/** A number is a signal's number exactly: « 9 » is SIGKILL, not 19 nor 29. */
export function matchesSignal(row: SignalRow, query: string): boolean {
  const wanted = query.trim();
  return DIGITS.test(wanted) ? holds(row.signal.linux, Number(wanted)) : matchesSearchable(row, query);
}

export const SIGNALS_REFERENCE: ReferenceSource<SignalWords, SignalRow> = {
  words: {
    fr: () => import('./signals.fr.json'),
    en: () => import('./signals.en.json'),
  },
  /** The action is searched too: « core » finds every signal that leaves a core dump. */
  rows: (words, label) =>
    SIGNALS.map((signal) => {
      const use = words.signals[signal.name] ?? '';
      return {
        key: signal.name,
        group: signal.group,
        signal,
        use,
        macos: signal.macos === undefined ? signal.linux : signal.macos,
        searchable: [
          signal.name,
          use,
          label(`tools.signals.groups.${signal.group}`),
          label(`tools.signals.actions.${signal.action}`),
        ],
      };
    }),
  title: (row) => (row.signal.linux === null ? row.key : `${row.key} (${row.signal.linux})`),
  line: (row) => row.use,
  matches: matchesSignal,
};
