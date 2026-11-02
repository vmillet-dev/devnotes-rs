import { Editor, getExtensionField } from '@tiptap/core';
import { afterEach, describe, it } from 'vitest';
import en from '@core/services/i18n/translations/en.json';
import fr from '@core/services/i18n/translations/fr.json';
import { GoldenCase, writeGolden, writingGolden } from '@testing/golden';
import { escapeMarkdownText } from './markdown-text';
import { RichEditorHooks, createRichEditor, markdownOf } from './rich-text.engine';

const HOOKS: RichEditorHooks = {
  label: 'Note',
  placeholder: '',
  change: () => undefined,
  blur: () => undefined,
  transaction: () => undefined,
  paste: () => false,
  click: () => false,
  keydown: () => false,
  indent: () => '  ',
  diagrams: { draw: () => new Promise(() => undefined), title: () => '', failed: () => '' },
  embeds: { picture: () => Promise.resolve(null), missing: () => '' },
};

/** Every GFM construction, those the schema does not hold, and what the escaping must keep. */
const MARKDOWN: readonly string[] = [
  '# Titre\n\n## Sous-titre\n\n### Trois',
  'Du **gras**, de l’*italique*, du ~~barré~~ et du `code`.',
  '- un\n- deux\n  - deux bis\n- trois',
  '1. un\n2. deux\n3. trois',
  '- [ ] à faire\n- [x] fait',
  '> Une citation\n> sur deux lignes',
  '```sql\nSELECT * FROM notes;\n```',
  '```mermaid\nflowchart LR\n  a --> b\n```',
  '| a | b |\n| --- | --- |\n| 1 | 2 |',
  '[un lien](https://exemple.fr) et https://nu.exemple.fr',
  '![image](devnotes:drawing/abc)',
  '---',
  'Ligne un  \nligne deux',
  'Un saut\\\ndur',
  '<div>du HTML brut</div>',
  'Une note[^1].\n\n[^1]: en bas de page',
  'Titre souligné\n==============',
  'Sous-titre souligné\n-------------------',
  '[référence][ref]\n\n[ref]: https://exemple.fr',
  'Un {{champ}} et {{autre_champ}} au milieu',
  '2 * 3 * 4',
  'Une tabulation &#9; codée',
  '*étoile* _souligné_ __gras__',
  '\\# pas un titre',
];

/** Text typed in the editor, as `escapeMarkdownText` writes it, at a line's start or not. */
const TYPED: readonly string[] = [
  '# pas un titre',
  '2 * 3 * 4',
  '1. pas une liste',
  '- pas une puce',
  '> pas une citation',
  'un *mot* et un _autre_',
  '[crochets] et (parenthèses)',
  'un {{champ}} *gardé*',
  'C:\\Users\\moi',
  '`backtick` et ~tilde~',
  '| a | b |',
  '<balise>',
  '+ plus',
  '***',
];

/** What a page and a code editor put on the clipboard. */
const PASTED_HTML: readonly string[] = [
  '<p>Du <b>gras</b>, de l’<i>italique</i> et <a href="https://exemple.fr">un lien</a>.</p>',
  '<h1>Titre</h1><h2>Sous-titre</h2><p>Texte</p>',
  '<ul><li>un</li><li>deux<ul><li>deux bis</li></ul></li></ul>',
  '<ol><li>un</li><li>deux</li></ol>',
  '<blockquote><p>Une citation</p></blockquote>',
  '<table><tr><th>a</th><th>b</th></tr><tr><td>1</td><td>2</td></tr></table>',
  '<pre><code>npm run build</code></pre>',
  '<pre><code><b>npm</b> run build</code></pre>',
  '<div style="font-family: monospace">const a = 1;<br>const b = 2;</div>',
  '<p><span style="font-weight:700">gras par style</span></p>',
  '<meta charset="utf-8"><p>Copié depuis {{une page}}</p>',
];

function samples(): string[] {
  const contents = (translations: Record<string, unknown>): string[] => {
    const notes = (translations['notes'] as Record<string, Record<string, unknown>>)['samples'];
    return Object.entries(notes)
      .filter(([key, value]) => key.endsWith('Content') && typeof value === 'string')
      .map(([, value]) => value as string);
  };
  return [...contents(fr), ...contents(en)];
}

let editors: Editor[] = [];

function editor(markdown: string): Editor {
  const element = document.createElement('div');
  document.body.append(element);
  const made = createRichEditor(element, markdown, HOOKS);
  editors.push(made);
  return made;
}

describe.runIf(writingGolden)('golden: rich text', () => {
  afterEach(() => {
    editors.forEach((made) => made.destroy());
    editors = [];
    document.body.replaceChildren();
  });

  it('captures Markdown read into the editor and written back', () => {
    const cases: GoldenCase[] = [...samples(), ...MARKDOWN].map((input) => ({
      input,
      output: markdownOf(editor(input)),
    }));
    writeGolden('markdown', cases);
  });

  it('captures typed text as it is stored', () => {
    const cases: GoldenCase[] = TYPED.flatMap((input) =>
      [false, true].map((startsLine) => ({
        input,
        options: { startsLine },
        output: escapeMarkdownText(input, startsLine),
      })),
    );
    writeGolden('markdown-escape', cases);
  });

  it('captures HTML pasted into an empty Note as the Markdown it becomes', () => {
    const cases: GoldenCase[] = PASTED_HTML.map((input) => {
      const made = editor('');
      made.view.pasteHTML(input);
      return { input, output: markdownOf(made) };
    });
    writeGolden('html-paste', cases);
  });

  it('captures the keyboard shortcuts and input rules of every extension', () => {
    const made = editor('');
    const cases: GoldenCase[] = made.extensionManager.extensions.map((extension) => {
      const context = {
        name: extension.name,
        options: extension.options,
        storage: extension.storage,
        editor: made,
        type: made.schema.nodes[extension.name] ?? made.schema.marks[extension.name],
        parent: undefined,
      };
      const shortcuts = getExtensionField<(() => Record<string, unknown>) | undefined>(
        extension,
        'addKeyboardShortcuts',
        context,
      );
      const rules = getExtensionField<(() => { find: unknown }[]) | undefined>(extension, 'addInputRules', context);
      return {
        input: extension.name,
        output: {
          shortcuts: Object.keys(shortcuts?.() ?? {}).sort(),
          inputRules: (rules?.() ?? []).map((rule) => String(rule.find)),
        },
      };
    });
    writeGolden('rich-keys', cases);
  });
});
