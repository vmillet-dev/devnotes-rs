export const MIME_GROUPS = ['text', 'application', 'image', 'audio', 'video', 'font', 'multipart'] as const;
export type MimeGroup = (typeof MIME_GROUPS)[number];

export interface MimeType {
  readonly type: string;
  /** With their dot, the most common first; none for a type that never sits in a file. */
  readonly extensions: readonly string[];
}

export interface MimeTypeWords {
  readonly types: Readonly<Record<string, string>>;
}

export function mimeGroup(type: string): MimeGroup {
  return type.slice(0, type.indexOf('/')) as MimeGroup;
}

const OFFICE = 'application/vnd.openxmlformats-officedocument';

export const MIME_TYPES: readonly MimeType[] = [
  { type: 'text/plain', extensions: ['.txt', '.text', '.log'] },
  { type: 'text/html', extensions: ['.html', '.htm'] },
  { type: 'text/css', extensions: ['.css'] },
  { type: 'text/javascript', extensions: ['.js', '.mjs', '.cjs'] },
  { type: 'text/csv', extensions: ['.csv'] },
  { type: 'text/tab-separated-values', extensions: ['.tsv'] },
  { type: 'text/markdown', extensions: ['.md', '.markdown'] },
  { type: 'text/calendar', extensions: ['.ics'] },
  { type: 'text/vcard', extensions: ['.vcf', '.vcard'] },
  { type: 'text/xml', extensions: ['.xml'] },
  { type: 'text/event-stream', extensions: [] },

  { type: 'application/json', extensions: ['.json'] },
  { type: 'application/ld+json', extensions: ['.jsonld'] },
  { type: 'application/geo+json', extensions: ['.geojson'] },
  { type: 'application/manifest+json', extensions: ['.webmanifest'] },
  { type: 'application/problem+json', extensions: [] },
  { type: 'application/x-ndjson', extensions: ['.ndjson', '.jsonl'] },
  { type: 'application/xml', extensions: ['.xml', '.xsd', '.xsl'] },
  { type: 'application/xhtml+xml', extensions: ['.xhtml'] },
  { type: 'application/rss+xml', extensions: ['.rss'] },
  { type: 'application/atom+xml', extensions: ['.atom'] },
  { type: 'application/yaml', extensions: ['.yaml', '.yml'] },
  { type: 'application/toml', extensions: ['.toml'] },
  { type: 'application/sql', extensions: ['.sql'] },
  { type: 'application/wasm', extensions: ['.wasm'] },
  { type: 'application/pdf', extensions: ['.pdf'] },
  { type: 'application/rtf', extensions: ['.rtf'] },
  { type: 'application/epub+zip', extensions: ['.epub'] },
  { type: 'application/msword', extensions: ['.doc'] },
  { type: `${OFFICE}.wordprocessingml.document`, extensions: ['.docx'] },
  { type: 'application/vnd.ms-excel', extensions: ['.xls'] },
  { type: `${OFFICE}.spreadsheetml.sheet`, extensions: ['.xlsx'] },
  { type: 'application/vnd.ms-powerpoint', extensions: ['.ppt'] },
  { type: `${OFFICE}.presentationml.presentation`, extensions: ['.pptx'] },
  { type: 'application/vnd.oasis.opendocument.text', extensions: ['.odt'] },
  { type: 'application/vnd.oasis.opendocument.spreadsheet', extensions: ['.ods'] },
  { type: 'application/zip', extensions: ['.zip'] },
  { type: 'application/gzip', extensions: ['.gz', '.tgz'] },
  { type: 'application/x-tar', extensions: ['.tar'] },
  { type: 'application/x-bzip2', extensions: ['.bz2'] },
  { type: 'application/x-xz', extensions: ['.xz'] },
  { type: 'application/zstd', extensions: ['.zst'] },
  { type: 'application/x-7z-compressed', extensions: ['.7z'] },
  { type: 'application/vnd.rar', extensions: ['.rar'] },
  { type: 'application/java-archive', extensions: ['.jar'] },
  { type: 'application/vnd.android.package-archive', extensions: ['.apk'] },
  { type: 'application/vnd.microsoft.portable-executable', extensions: ['.exe', '.dll'] },
  { type: 'application/vnd.debian.binary-package', extensions: ['.deb'] },
  { type: 'application/x-sh', extensions: ['.sh'] },
  { type: 'application/vnd.ms-fontobject', extensions: ['.eot'] },
  { type: 'application/octet-stream', extensions: ['.bin'] },
  { type: 'application/x-www-form-urlencoded', extensions: [] },
  { type: 'application/jwt', extensions: [] },

  { type: 'image/png', extensions: ['.png'] },
  { type: 'image/jpeg', extensions: ['.jpg', '.jpeg', '.jfif'] },
  { type: 'image/gif', extensions: ['.gif'] },
  { type: 'image/webp', extensions: ['.webp'] },
  { type: 'image/avif', extensions: ['.avif'] },
  { type: 'image/apng', extensions: ['.apng'] },
  { type: 'image/svg+xml', extensions: ['.svg'] },
  { type: 'image/vnd.microsoft.icon', extensions: ['.ico'] },
  { type: 'image/bmp', extensions: ['.bmp'] },
  { type: 'image/tiff', extensions: ['.tif', '.tiff'] },
  { type: 'image/heic', extensions: ['.heic'] },

  { type: 'audio/mpeg', extensions: ['.mp3'] },
  { type: 'audio/aac', extensions: ['.aac'] },
  { type: 'audio/mp4', extensions: ['.m4a'] },
  { type: 'audio/ogg', extensions: ['.ogg', '.oga'] },
  { type: 'audio/opus', extensions: ['.opus'] },
  { type: 'audio/wav', extensions: ['.wav'] },
  { type: 'audio/flac', extensions: ['.flac'] },
  { type: 'audio/webm', extensions: ['.weba'] },
  { type: 'audio/midi', extensions: ['.mid', '.midi'] },

  { type: 'video/mp4', extensions: ['.mp4', '.m4v'] },
  { type: 'video/webm', extensions: ['.webm'] },
  { type: 'video/ogg', extensions: ['.ogv'] },
  { type: 'video/quicktime', extensions: ['.mov'] },
  { type: 'video/x-msvideo', extensions: ['.avi'] },
  { type: 'video/x-matroska', extensions: ['.mkv'] },
  { type: 'video/mpeg', extensions: ['.mpeg', '.mpg'] },
  { type: 'video/mp2t', extensions: ['.ts', '.m2ts'] },
  { type: 'video/3gpp', extensions: ['.3gp'] },

  { type: 'font/woff2', extensions: ['.woff2'] },
  { type: 'font/woff', extensions: ['.woff'] },
  { type: 'font/ttf', extensions: ['.ttf'] },
  { type: 'font/otf', extensions: ['.otf'] },
  { type: 'font/collection', extensions: ['.ttc'] },

  { type: 'multipart/form-data', extensions: [] },
  { type: 'multipart/byteranges', extensions: [] },
  { type: 'multipart/mixed', extensions: [] },
  { type: 'multipart/alternative', extensions: [] },
];
