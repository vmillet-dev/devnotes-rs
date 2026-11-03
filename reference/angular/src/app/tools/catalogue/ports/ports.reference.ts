import { ReferenceRow, ReferenceSource, matchesSearchable } from '@core/services/tools/reference.model';
import { PORTS, Port, PortWords, portRange } from './ports.data';

export interface PortRow extends ReferenceRow {
  readonly port: Port;
  readonly note: string;
}

const DIGITS = /^\d+$/;

/** A number is read as the start of a port, so that « 54 » finds 5432 and not 1521's note. */
export function matchesPort(row: PortRow, query: string): boolean {
  const wanted = query.trim();
  return DIGITS.test(wanted) ? row.key.startsWith(wanted) : matchesSearchable(row, query);
}

export const PORTS_REFERENCE: ReferenceSource<PortWords, PortRow> = {
  words: {
    fr: () => import('./ports.fr.json'),
    en: () => import('./ports.en.json'),
  },
  rows: (words, label) =>
    PORTS.map((port) => {
      const note = words.ports[port.number] ?? '';
      const range = portRange(port.number);
      return {
        key: String(port.number),
        group: range,
        port,
        note,
        searchable: [String(port.number), port.service, note, label(`tools.ports.ranges.${range}`)],
      };
    }),
  title: (row) => `${row.key} ${row.port.service}`,
  line: (row) => row.note,
  matches: matchesPort,
};
