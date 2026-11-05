// Copies Prettier, as `package.json` pins it, into crates/text/prettier: the formatter the
// application runs in QuickJS (`text::format`, plan D1). `npm run prettier` after a bump.
import { copyFileSync, mkdirSync, readFileSync, rmSync } from 'node:fs';
import { join } from 'node:path';

const FROM = join('node_modules', 'prettier');
const TO = join('crates', 'text', 'prettier');
const PLUGINS = [
  'angular',
  'babel',
  'estree',
  'graphql',
  'html',
  'markdown',
  'postcss',
  'typescript',
  'yaml',
];

rmSync(TO, { recursive: true, force: true });
mkdirSync(TO, { recursive: true });
copyFileSync(join(FROM, 'standalone.js'), join(TO, 'standalone.js'));
for (const plugin of PLUGINS) copyFileSync(join(FROM, 'plugins', `${plugin}.js`), join(TO, `${plugin}.js`));
copyFileSync(join(FROM, 'LICENSE'), join(TO, 'LICENSE'));

const { version } = JSON.parse(readFileSync(join(FROM, 'package.json'), 'utf8'));
console.log(`Prettier ${version}: standalone and ${PLUGINS.length} plugins`);
