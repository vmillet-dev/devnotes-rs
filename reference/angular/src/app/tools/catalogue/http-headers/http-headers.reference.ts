import { ReferenceRow, ReferenceSource } from '@core/services/tools/reference.model';
import { HTTP_HEADERS, HttpHeader, HttpHeaderWords } from './http-headers.data';

export interface HeaderRow extends ReferenceRow {
  readonly header: HttpHeader;
  readonly does: string;
}

export const HTTP_HEADERS_REFERENCE: ReferenceSource<HttpHeaderWords, HeaderRow> = {
  words: {
    fr: () => import('./http-headers.fr.json'),
    en: () => import('./http-headers.en.json'),
  },
  rows: (words, label) =>
    HTTP_HEADERS.map((header) => {
      const does = words.headers[header.name.toLowerCase()] ?? '';
      return {
        key: header.name,
        group: header.group,
        header,
        does,
        searchable: [
          header.name,
          does,
          header.example,
          header.by,
          label(`tools.http-headers.groups.${header.group}`),
        ],
      };
    }),
  title: (row) => row.key,
  line: (row) => row.does,
};
