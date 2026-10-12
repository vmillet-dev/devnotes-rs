import { ReferenceRow, ReferenceSource } from '@core/services/tools/reference.model';
import { MIME_TYPES, MimeType, MimeTypeWords, mimeGroup } from './mime-types.data';

export interface MimeRow extends ReferenceRow {
  readonly mime: MimeType;
  readonly description: string;
}

export const MIME_TYPES_REFERENCE: ReferenceSource<MimeTypeWords, MimeRow> = {
  words: {
    fr: () => import('./mime-types.fr.json'),
    en: () => import('./mime-types.en.json'),
  },
  /** `.webp` and `webp` both find image/webp: each extension is searched with its dot and without. */
  rows: (words) =>
    MIME_TYPES.map((mime) => {
      const description = words.types[mime.type] ?? '';
      return {
        key: mime.type,
        group: mimeGroup(mime.type),
        mime,
        description,
        searchable: [
          mime.type,
          description,
          ...mime.extensions,
          ...mime.extensions.map((extension) => extension.slice(1)),
        ],
      };
    }),
  title: (row) => row.key,
  line: (row) => row.description,
};
