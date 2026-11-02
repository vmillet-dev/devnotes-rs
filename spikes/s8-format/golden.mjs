// Writes Prettier 3.9.10's answer for every fixture into fixtures/golden/, with the options
// the application passes (prettierOptions(DEFAULT_PRETTIER_SETTINGS, '  ')), endOfLine auto.
import { readFileSync, writeFileSync, mkdirSync, readdirSync } from 'node:fs';
import { createRequire } from 'node:module';
const require = createRequire(process.argv[2] + '/');
const prettier = require('prettier');
const options = { printWidth: 100, tabWidth: 2, useTabs: false, singleQuote: true, semi: true, trailingComma: 'all', endOfLine: 'auto' };
const parsers = { ts: 'typescript', js: 'babel', json: 'json', css: 'css', scss: 'scss', less: 'less', html: 'html', vue: 'vue', md: 'markdown', yaml: 'yaml', graphql: 'graphql', angular: 'angular' };
mkdirSync('fixtures/golden', { recursive: true });
for (const file of readdirSync('fixtures').filter((f) => f.includes('.'))) {
  const parser = parsers[file.split('.').pop()];
  const text = readFileSync('fixtures/' + file, 'utf8');
  try {
    writeFileSync('fixtures/golden/' + file, await prettier.format(text, { ...options, parser }));
  } catch (e) {
    writeFileSync('fixtures/golden/' + file, 'ERROR ' + e.message.split('\n')[0]);
  }
}
