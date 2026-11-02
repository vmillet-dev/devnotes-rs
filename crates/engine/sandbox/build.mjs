// Bundles what a request script can `require` into dist/, one file a module, which the
// application compiles in (`http::scripts::libraries`). Run again after changing a version:
// `npm ci --ignore-scripts && npm run build`, from this folder.

import { build } from 'esbuild';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const out = join(here, 'dist');

// Node's own modules a Postman script reaches, each a polyfill of its own. Left out of every
// other bundle: they are required at run time, so one copy serves them all.
const BUILTINS = {
  assert: 'assert',
  buffer: 'buffer',
  events: 'events',
  path: 'path-browserify',
  punycode: 'punycode',
  querystring: 'querystring-es3',
  stream: 'stream-browserify',
  string_decoder: 'string_decoder',
  timers: 'timers-browserify',
  url: 'url',
  util: 'util',
};

const LIBRARIES = {
  ajv: 'ajv',
  chai: 'chai',
  // Without `fromURL` and parse5: the slim build reads HTML with htmlparser2 alone.
  cheerio: 'cheerio/slim',
  'crypto-js': 'crypto-js',
  'csv-parse': 'csv-parse',
  'csv-parse/sync': 'csv-parse/sync',
  lodash: 'lodash',
  moment: 'moment',
  tv4: 'tv4',
  uuid: 'uuid',
  xml2js: 'xml2js',
};

const fileOf = (name) => `${name.replace(/[^a-z0-9]+/g, '-')}.js`;

function packageOf(input) {
  // `(disabled):…` is a file a package's `browser` field replaces with nothing.
  if (input.startsWith('(')) return null;
  const at = input.lastIndexOf('node_modules/');
  if (at < 0) return null;
  const parts = input.slice(at + 'node_modules/'.length).split('/');
  const name = parts[0].startsWith('@') ? `${parts[0]}/${parts[1]}` : parts[0];
  return join(here, input.slice(0, at), 'node_modules', name);
}

function licenceOf(root) {
  const manifest = JSON.parse(readFileSync(join(root, 'package.json'), 'utf8'));
  const file = readdirSync(root).find((entry) => /^(licen[cs]e|copying)/i.test(entry));
  return {
    name: manifest.name,
    version: manifest.version,
    licence: typeof manifest.license === 'string' ? manifest.license : JSON.stringify(manifest.license),
    text: file === undefined ? '' : readFileSync(join(root, file), 'utf8').trim(),
  };
}

async function bundle(name, entry, external) {
  const result = await build({
    stdin: { contents: `module.exports = require(${JSON.stringify(entry)});`, resolveDir: here },
    bundle: true,
    format: 'cjs',
    platform: 'browser',
    target: 'es2022',
    minify: true,
    legalComments: 'none',
    metafile: true,
    write: false,
    external: [...external.filter((builtin) => builtin !== name), 'node:*', 'crypto'],
    define: { global: 'globalThis', 'process.env.NODE_ENV': '"production"' },
    logLevel: 'error',
  });
  return { text: result.outputFiles[0].text, inputs: Object.keys(result.metafile.inputs) };
}

if (existsSync(out)) rmSync(out, { recursive: true });
mkdirSync(out);

const builtins = Object.keys(BUILTINS);
const manifest = [];
const packages = new Map();
for (const [name, entry] of [...Object.entries(BUILTINS), ...Object.entries(LIBRARIES)]) {
  const { text, inputs } = await bundle(name, entry, builtins);
  const file = fileOf(name);
  writeFileSync(join(out, file), text);
  manifest.push({
    name,
    file,
    bytes: Buffer.byteLength(text),
    sha256: createHash('sha256').update(text).digest('hex'),
  });
  for (const root of inputs.map(packageOf).filter((root) => root !== null)) {
    if (!packages.has(root)) packages.set(root, licenceOf(root));
  }
}

writeFileSync(join(out, 'manifest.json'), `${JSON.stringify(manifest, null, 2)}\n`);
const notices = [...packages.values()]
  .sort((a, b) => a.name.localeCompare(b.name))
  .map((pkg) => `${pkg.name} ${pkg.version} (${pkg.licence})\n\n${pkg.text}`)
  .join(`\n\n${'-'.repeat(72)}\n\n`);
writeFileSync(join(out, 'THIRD-PARTY-LICENSES.txt'), `${notices}\n`);

const total = manifest.reduce((sum, entry) => sum + entry.bytes, 0);
console.log(`${manifest.length} modules, ${packages.size} packages, ${(total / 1024).toFixed(0)} KB`);
