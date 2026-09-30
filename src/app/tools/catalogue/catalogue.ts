import { ToolDefinition } from '@core/services/tools/tool.model';

/**
 * Every tool, in its panel's order, each in a folder beside this one. Adding one is an entry
 * here: its name and its line are read from its id, and its component is a chunk of its own,
 * loaded the first time it is opened.
 */
export const TOOLS: readonly ToolDefinition[] = [
  {
    id: 'case',
    category: 'text',
    keywords: [
      'camelCase',
      'PascalCase',
      'snake_case',
      'kebab-case',
      'CONSTANT_CASE',
      'dot.case',
      'path/case',
      'Train-Case',
      'lowercase',
      'UPPERCASE',
      'flatcase',
    ],
    load: () => import('./case/case-tool.component').then((m) => m.CaseToolComponent),
  },
  {
    id: 'slug',
    category: 'text',
    keywords: ['slug', 'permalink', 'ascii'],
    load: () => import('./slug/slug-tool.component').then((m) => m.SlugToolComponent),
  },
  {
    id: 'url-parser',
    category: 'text',
    keywords: ['url', 'uri', 'query', 'querystring'],
    load: () => import('./url-parser/url-parser-tool.component').then((m) => m.UrlParserToolComponent),
  },
  {
    id: 'line-breaks',
    category: 'text',
    keywords: ['crlf', 'lf', 'eol', 'newline', 'whitespace'],
    load: () => import('./line-breaks/line-breaks-tool.component').then((m) => m.LineBreaksToolComponent),
  },
  {
    id: 'hash',
    category: 'crypto',
    keywords: ['md5', 'sha1', 'sha256', 'sha512', 'sha3', 'hmac', 'digest', 'checksum', 'webhook'],
    load: () => import('./hash/hash-tool.component').then((m) => m.HashToolComponent),
  },
  {
    id: 'password',
    category: 'crypto',
    keywords: ['password', 'passphrase', 'secret', 'random', 'entropy'],
    load: () => import('./password/password-tool.component').then((m) => m.PasswordToolComponent),
  },
  {
    id: 'uuid',
    category: 'crypto',
    keywords: ['uuid', 'guid', 'v4', 'v7', 'identifier'],
    load: () => import('./uuid/uuid-tool.component').then((m) => m.UuidToolComponent),
  },
  {
    id: 'jwt',
    category: 'crypto',
    keywords: ['jwt', 'json web token', 'bearer', 'hs256', 'claims', 'exp', 'token'],
    load: () => import('./jwt/jwt-tool.component').then((m) => m.JwtToolComponent),
  },
  {
    id: 'url-codec',
    category: 'encode',
    keywords: ['url', 'percent', 'encodeURIComponent', 'escape'],
    load: () => import('./url-codec/url-codec-tool.component').then((m) => m.UrlCodecToolComponent),
  },
  {
    id: 'base64',
    category: 'encode',
    keywords: ['base64', 'b64', 'jwt', 'data uri'],
    load: () => import('./base64/base64-tool.component').then((m) => m.Base64ToolComponent),
  },
  {
    id: 'convert',
    category: 'encode',
    keywords: ['json', 'toml', 'xml', 'yaml', 'yml', 'convert'],
    load: () => import('./convert/convert-tool.component').then((m) => m.ConvertToolComponent),
  },
  {
    id: 'colour',
    category: 'encode',
    keywords: ['color', 'hex', 'rgb', 'hsl', 'oklch', 'contrast', 'wcag'],
    load: () => import('./colour/colour-tool.component').then((m) => m.ColourToolComponent),
  },
  {
    id: 'json-diff',
    category: 'compare',
    keywords: ['json', 'diff', 'compare', 'patch', 'rfc 6902'],
    load: () => import('./json-diff/json-diff-tool.component').then((m) => m.JsonDiffToolComponent),
  },
  {
    id: 'json-generator',
    category: 'generate',
    keywords: ['json', 'schema', 'mock', 'fixture', 'fake', 'faker'],
    load: () =>
      import('./json-generator/json-generator-tool.component').then((m) => m.JsonGeneratorToolComponent),
  },
  {
    id: 'lorem',
    category: 'generate',
    keywords: ['lorem', 'ipsum', 'placeholder', 'text'],
    load: () => import('./lorem/lorem-tool.component').then((m) => m.LoremToolComponent),
  },
  {
    id: 'dates',
    category: 'time',
    keywords: ['timestamp', 'unix', 'epoch', 'iso 8601', 'rfc 3339', 'rfc 2822', 'date', 'now'],
    load: () => import('./dates/dates-tool.component').then((m) => m.DatesToolComponent),
  },
  {
    id: 'zones',
    category: 'time',
    keywords: ['timezone', 'time zone', 'tz', 'iana', 'utc', 'gmt', 'dst', 'offset'],
    load: () => import('./zones/zones-tool.component').then((m) => m.ZonesToolComponent),
  },
  {
    id: 'cron',
    category: 'time',
    keywords: ['cron', 'crontab', 'schedule', 'quartz', '@daily', 'job'],
    load: () => import('./cron/cron-tool.component').then((m) => m.CronToolComponent),
  },
  {
    id: 'sizes',
    category: 'calc',
    keywords: ['byte', 'octet', 'kb', 'mb', 'gb', 'kib', 'mib', 'gib', 'bit', 'mbit', 'size'],
    load: () => import('./sizes/sizes-tool.component').then((m) => m.SizesToolComponent),
  },
  {
    id: 'percentages',
    category: 'calc',
    keywords: ['percent', 'percentage', '%', 'pourcent', 'ratio', 'discount', 'vat', 'tva'],
    load: () => import('./percentages/percentages-tool.component').then((m) => m.PercentagesToolComponent),
  },
  {
    id: 'permissions',
    category: 'calc',
    keywords: ['chmod', 'umask', 'rwx', 'octal', 'setuid', 'setgid', 'sticky', 'ls -l'],
    load: () => import('./permissions/permissions-tool.component').then((m) => m.PermissionsToolComponent),
  },
  {
    id: 'checks',
    category: 'calc',
    keywords: ['luhn', 'iban', 'mod 97', 'credit card', 'carte', 'checksum', 'bic', 'visa', 'mastercard'],
    load: () => import('./checks/checks-tool.component').then((m) => m.ChecksToolComponent),
  },
];
