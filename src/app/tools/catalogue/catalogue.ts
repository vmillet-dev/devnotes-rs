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
    keywords: ['camelCase', 'PascalCase', 'snake_case', 'kebab-case', 'CONSTANT_CASE'],
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
];
