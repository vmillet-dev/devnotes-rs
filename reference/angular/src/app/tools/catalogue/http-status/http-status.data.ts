export const STATUS_CLASSES = ['1xx', '2xx', '3xx', '4xx', '5xx'] as const;
export type StatusClass = (typeof STATUS_CLASSES)[number];

export interface HttpStatus {
  readonly code: number;
  /** The reason phrase as specified, in English whatever the language on screen. */
  readonly name: string;
  /** The RFC that defines it, or the software that sends it. */
  readonly by: string;
  /** Outside the IANA registry, or dropped from it. */
  readonly standing?: 'unofficial' | 'obsolete';
}

export interface HttpStatusWords {
  readonly codes: Readonly<Record<string, { readonly meaning: string; readonly use?: string }>>;
}

export function statusClass(code: number): StatusClass {
  return `${Math.floor(code / 100)}xx` as StatusClass;
}

const RFC = 'RFC 9110';
const WEBDAV = 'RFC 4918';

export const HTTP_STATUSES: readonly HttpStatus[] = [
  { code: 100, name: 'Continue', by: RFC },
  { code: 101, name: 'Switching Protocols', by: RFC },
  { code: 102, name: 'Processing', by: 'RFC 2518', standing: 'obsolete' },
  { code: 103, name: 'Early Hints', by: 'RFC 8297' },
  { code: 200, name: 'OK', by: RFC },
  { code: 201, name: 'Created', by: RFC },
  { code: 202, name: 'Accepted', by: RFC },
  { code: 203, name: 'Non-Authoritative Information', by: RFC },
  { code: 204, name: 'No Content', by: RFC },
  { code: 205, name: 'Reset Content', by: RFC },
  { code: 206, name: 'Partial Content', by: RFC },
  { code: 207, name: 'Multi-Status', by: WEBDAV },
  { code: 208, name: 'Already Reported', by: 'RFC 5842' },
  { code: 226, name: 'IM Used', by: 'RFC 3229' },
  { code: 300, name: 'Multiple Choices', by: RFC },
  { code: 301, name: 'Moved Permanently', by: RFC },
  { code: 302, name: 'Found', by: RFC },
  { code: 303, name: 'See Other', by: RFC },
  { code: 304, name: 'Not Modified', by: RFC },
  { code: 305, name: 'Use Proxy', by: RFC, standing: 'obsolete' },
  { code: 307, name: 'Temporary Redirect', by: RFC },
  { code: 308, name: 'Permanent Redirect', by: RFC },
  { code: 400, name: 'Bad Request', by: RFC },
  { code: 401, name: 'Unauthorized', by: RFC },
  { code: 402, name: 'Payment Required', by: RFC },
  { code: 403, name: 'Forbidden', by: RFC },
  { code: 404, name: 'Not Found', by: RFC },
  { code: 405, name: 'Method Not Allowed', by: RFC },
  { code: 406, name: 'Not Acceptable', by: RFC },
  { code: 407, name: 'Proxy Authentication Required', by: RFC },
  { code: 408, name: 'Request Timeout', by: RFC },
  { code: 409, name: 'Conflict', by: RFC },
  { code: 410, name: 'Gone', by: RFC },
  { code: 411, name: 'Length Required', by: RFC },
  { code: 412, name: 'Precondition Failed', by: RFC },
  { code: 413, name: 'Content Too Large', by: RFC },
  { code: 414, name: 'URI Too Long', by: RFC },
  { code: 415, name: 'Unsupported Media Type', by: RFC },
  { code: 416, name: 'Range Not Satisfiable', by: RFC },
  { code: 417, name: 'Expectation Failed', by: RFC },
  { code: 418, name: "I'm a teapot", by: 'RFC 2324', standing: 'unofficial' },
  { code: 421, name: 'Misdirected Request', by: RFC },
  { code: 422, name: 'Unprocessable Content', by: RFC },
  { code: 423, name: 'Locked', by: WEBDAV },
  { code: 424, name: 'Failed Dependency', by: WEBDAV },
  { code: 425, name: 'Too Early', by: 'RFC 8470' },
  { code: 426, name: 'Upgrade Required', by: RFC },
  { code: 428, name: 'Precondition Required', by: 'RFC 6585' },
  { code: 429, name: 'Too Many Requests', by: 'RFC 6585' },
  { code: 431, name: 'Request Header Fields Too Large', by: 'RFC 6585' },
  { code: 440, name: 'Login Time-out', by: 'IIS', standing: 'unofficial' },
  { code: 444, name: 'No Response', by: 'nginx', standing: 'unofficial' },
  { code: 449, name: 'Retry With', by: 'IIS', standing: 'unofficial' },
  { code: 451, name: 'Unavailable For Legal Reasons', by: 'RFC 7725' },
  { code: 494, name: 'Request Header Too Large', by: 'nginx', standing: 'unofficial' },
  { code: 495, name: 'SSL Certificate Error', by: 'nginx', standing: 'unofficial' },
  { code: 496, name: 'SSL Certificate Required', by: 'nginx', standing: 'unofficial' },
  { code: 497, name: 'HTTP Request Sent to HTTPS Port', by: 'nginx', standing: 'unofficial' },
  { code: 499, name: 'Client Closed Request', by: 'nginx', standing: 'unofficial' },
  { code: 500, name: 'Internal Server Error', by: RFC },
  { code: 501, name: 'Not Implemented', by: RFC },
  { code: 502, name: 'Bad Gateway', by: RFC },
  { code: 503, name: 'Service Unavailable', by: RFC },
  { code: 504, name: 'Gateway Timeout', by: RFC },
  { code: 505, name: 'HTTP Version Not Supported', by: RFC },
  { code: 506, name: 'Variant Also Negotiates', by: 'RFC 2295' },
  { code: 507, name: 'Insufficient Storage', by: WEBDAV },
  { code: 508, name: 'Loop Detected', by: 'RFC 5842' },
  { code: 509, name: 'Bandwidth Limit Exceeded', by: 'Apache', standing: 'unofficial' },
  { code: 510, name: 'Not Extended', by: 'RFC 2774', standing: 'obsolete' },
  { code: 511, name: 'Network Authentication Required', by: 'RFC 6585' },
  { code: 520, name: 'Web Server Returned an Unknown Error', by: 'Cloudflare', standing: 'unofficial' },
  { code: 521, name: 'Web Server Is Down', by: 'Cloudflare', standing: 'unofficial' },
  { code: 522, name: 'Connection Timed Out', by: 'Cloudflare', standing: 'unofficial' },
  { code: 523, name: 'Origin Is Unreachable', by: 'Cloudflare', standing: 'unofficial' },
  { code: 524, name: 'A Timeout Occurred', by: 'Cloudflare', standing: 'unofficial' },
  { code: 525, name: 'SSL Handshake Failed', by: 'Cloudflare', standing: 'unofficial' },
  { code: 526, name: 'Invalid SSL Certificate', by: 'Cloudflare', standing: 'unofficial' },
];
