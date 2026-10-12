import { ReferenceRow, ReferenceSource } from '@core/services/tools/reference.model';
import { HTTP_STATUSES, HttpStatus, HttpStatusWords, statusClass } from './http-status.data';

export interface StatusRow extends ReferenceRow {
  readonly status: HttpStatus;
  readonly meaning: string;
  readonly use: string | null;
}

export const HTTP_STATUS_REFERENCE: ReferenceSource<HttpStatusWords, StatusRow> = {
  words: {
    fr: () => import('./http-status.fr.json'),
    en: () => import('./http-status.en.json'),
  },
  rows: (words, label) =>
    HTTP_STATUSES.map((status) => {
      const group = statusClass(status.code);
      const said = words.codes[status.code];
      const meaning = said?.meaning ?? '';
      const use = said?.use ?? null;
      return {
        key: String(status.code),
        group,
        status,
        meaning,
        use,
        searchable: [
          String(status.code),
          status.name,
          meaning,
          use ?? '',
          status.by,
          label(`tools.http-status.groups.${group}`),
        ],
      };
    }),
  title: (row) => `${row.key} ${row.status.name}`,
  line: (row) => row.meaning,
};
