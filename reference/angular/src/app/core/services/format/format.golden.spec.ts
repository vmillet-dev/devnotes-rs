import { describe, it } from 'vitest';
import { GoldenCase, writeGolden, writingGolden } from '@testing/golden';
import { DEFAULT_PRETTIER_SETTINGS, PrettierParser, PrettierSettings, prettierOptions } from './format.model';
import { loadPlugins } from './prettier-plugins';
import { runPrettier } from './prettier-runner';

/** One text a parser, a `{{field}}` in most: the shield is part of what is captured. */
const TEXTS: Readonly<Record<PrettierParser, readonly string[]>> = {
  babel: [
    'const a = {b:1}\nconst c = 2;',
    `call("${'a'.repeat(40)}", "${'b'.repeat(40)}", "${'c'.repeat(20)}")`,
    'fetch("{{baseUrl}}/users/{{id}}", {headers:{Authorization:"Bearer {{token}}"}})',
    'if (a) { b() }\n',
    'const broken = {',
  ],
  typescript: [
    'function keysOf<T extends object>(value:T){return Object.keys(value) as (keyof T)[]}',
    'const url = `{{host}}/api/${version}`;',
  ],
  json: ['{"a":1,"b":[1,2,3],"token":"{{token}}"}', '{"nested":{"deep":{"value":true}}}\n'],
  css: ['a{color:red}', 'a{color:red}\n', '.card{margin:{{gap}}px;padding:0 4px}'],
  scss: ['.a{.b{color:$main;&:hover{color:darken($main,10%)}}}'],
  less: ['@main:#333;.a{color:@main;.b{width:~"calc(100% - {{gutter}})"}}'],
  html: ['<div><p>Bonjour {{name}}</p><ul><li>un</li><li>deux</li></ul></div>'],
  vue: [
    '<template><div :class="{a:b}">{{ message }}</div></template>\n<script>export default {data(){return {message:"x"}}}</script>',
  ],
  angular: ['<div *ngIf="shown" (click)="toggle()">{{ label }}</div>'],
  markdown: ['# Titre\n\n* un\n* deux\n\n|a|b|\n|-|-|\n|1|2|\n\nVoir {{lien}}.'],
  yaml: ['a:   1\nb:\n    - x\n    - "{{value}}"\n'],
  graphql: ['query($id:ID!){user(id:$id){name,posts(first:{{count}}){title}}}'],
};

const SETTINGS: readonly {
  readonly name: string;
  readonly settings: PrettierSettings;
  readonly indent: string;
}[] = [
  { name: 'default, editor of two spaces', settings: DEFAULT_PRETTIER_SETTINGS, indent: '  ' },
  { name: 'default, editor of a tab', settings: DEFAULT_PRETTIER_SETTINGS, indent: '\t' },
];

describe.runIf(writingGolden)('golden: format', () => {
  it('captures what Prettier answers, every parser and setting', async () => {
    const cases: GoldenCase[] = [];
    for (const [parser, texts] of Object.entries(TEXTS) as [PrettierParser, readonly string[]][]) {
      for (const text of texts) {
        for (const { name, settings, indent } of SETTINGS) {
          const options = prettierOptions(settings, indent);
          const output = await runPrettier({ text, cursor: 0, parser, options }, loadPlugins);
          cases.push({ input: { parser, text }, options: { name, ...options }, output });
        }
      }
    }
    writeGolden('format', cases);
  }, 120_000);
});
