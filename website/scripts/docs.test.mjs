// One test per convention of docs/content/README.md, and one per thing it refuses.
import assert from 'node:assert/strict';
import { describe, it } from 'node:test';

import { readPage, readPages, slug } from './docs.mjs';

const HEAD = '# Titre\n\nRésumé.\n\n';
const follow = (href) => href;
const body = (markdown) => readPage('page', HEAD + markdown, follow).body;
const refused = (markdown, message) => assert.throws(() => readPage('page', markdown, follow), { message });

describe('a page', () => {
  it('opens on its title and its summary', () => {
    const page = readPage('page', '# Prise en main\n\nInstaller l’application.\n', follow);
    assert.deepEqual(page, {
      id: 'page',
      title: 'Prise en main',
      summary: 'Installer l’application.',
      body: [],
    });
  });

  it('is cut into sections, addressed by their text', () => {
    assert.deepEqual(body('## Dérivation de clé\n\n### L’export\n'), [
      { kind: 'h2', id: 'derivation-de-cle', text: 'Dérivation de clé' },
      { kind: 'h3', id: 'lexport', text: 'L’export' },
    ]);
  });

  it('reads a label, a shortcut, code, bold and a link inside a sentence', () => {
    const [{ text }] = body(
      'Faites *Fichier → Préférences…* (<kbd>Ctrl Alt P</kbd>), `{{host}}`, **tout** ou [là](https://a.fr).',
    );
    assert.deepEqual(text, [
      'Faites ',
      { ui: 'Fichier → Préférences…' },
      ' (',
      { keys: ['Ctrl', 'Alt', 'P'] },
      '), ',
      { code: '{{host}}' },
      ', ',
      { strong: 'tout' },
      ' ou ',
      { link: 'là', to: 'https://a.fr' },
      '.',
    ]);
  });

  it('keeps an escaped character as itself, in the sentence it cuts', () => {
    assert.deepEqual(body('2 \\* 3, `<T>` et \\<T\\>')[0].text, ['2 * 3, ', { code: '<T>' }, ' et <T>']);
  });

  it('reads lists, numbered or not', () => {
    assert.deepEqual(body('- un\n- *deux*\n'), [
      { kind: 'list', ordered: false, items: [['un'], [{ ui: 'deux' }]] },
    ]);
    assert.equal(body('1. un\n2. deux\n')[0].ordered, true);
  });

  it('reads a list of nothing but pages as the list of them', () => {
    assert.deepEqual(body('- [Écrire](writing.md)\n- [Ranger](organising.md)\n'), [
      { kind: 'guides', pages: ['writing', 'organising'] },
    ]);
    assert.equal(body('- [Écrire](writing.md#titre)\n')[0].kind, 'list');
  });

  it('reads tips and warnings', () => {
    assert.deepEqual(body('> [!TIP]\n> Un *conseil*.\n'), [
      { kind: 'note', tone: 'tip', text: ['Un ', { ui: 'conseil' }, '.'] },
    ]);
    assert.deepEqual(body('> [!WARNING]\n> **Attention.** Voilà.\n'), [
      { kind: 'note', tone: 'warn', text: [{ strong: 'Attention.' }, ' Voilà.'] },
    ]);
  });

  it('titles a table and a code block by the bold line above them', () => {
    assert.deepEqual(body('**Fichiers**\n\n| Système | Fichier |\n| --- | --- |\n| Linux | `a\\|b` |\n'), [
      {
        kind: 'table',
        caption: 'Fichiers',
        head: ['Système', 'Fichier'],
        rows: [[['Linux'], [{ code: 'a|b' }]]],
      },
    ]);
    assert.deepEqual(body('**Installer**\n\n```sh\nsudo apt install\n```\n'), [
      { kind: 'code', title: 'Installer', lang: 'sh', code: 'sudo apt install' },
    ]);
  });

  it('reads a table headed by the keys as shortcuts, in both languages', () => {
    for (const head of ['Touches', 'Keys']) {
      assert.deepEqual(
        body(`**Palette**\n\n| ${head} | Action |\n| --- | --- |\n| <kbd>↑ ↓</kbd> | Choisir |\n`),
        [{ kind: 'keys', caption: 'Palette', rows: [{ keys: ['↑', '↓'], does: 'Choisir' }] }],
      );
    }
  });

  it('keeps as a paragraph a bold line that titles nothing', () => {
    assert.deepEqual(body('**Seul.**\n\nSuite.\n')[0], { kind: 'p', text: [{ strong: 'Seul.' }] });
  });
});

describe('what is refused', () => {
  it('names the file and the line', () => {
    refused(`${HEAD}Ligne cinq.\n\nEt *sept.\n`, /^page\.md:7: « \* » is left in « Et \*sept\. »/);
  });

  it('refuses a page without its title or its summary', () => {
    refused('Sans titre.\n', /begins with # and its title/);
    refused('# Titre\n\n- liste\n', /followed by one paragraph/);
    refused('# Titre\n', /a title and a summary/);
  });

  it('refuses a table or a code block without a title', () => {
    refused(`${HEAD}| A |\n| --- |\n| b |\n`, /a table has its title/);
    refused(`${HEAD}\`\`\`sh\nls\n\`\`\`\n`, /a code block has its title/);
  });

  it('refuses what no reader draws', () => {
    refused(`${HEAD}#### Trop bas\n`, /## or ###/);
    refused(`${HEAD}![image](a.png)\n`, /image is not read/);
    refused(`${HEAD}<b>gras</b>\n`, /is not read|is left/);
    refused(`${HEAD}---\n`, /hr is not read/);
    refused(`${HEAD}> Une citation.\n`, /begins with \[!TIP\]/);
    refused(`${HEAD}- [ ] à faire\n`, /task list/);
    refused(`${HEAD}- un\n  - dedans\n`, /holds one paragraph/);
    refused(
      `${HEAD}**Clés**\n\n| Touches | Action |\n| --- | --- |\n| Ctrl | Choisir |\n`,
      /one <kbd> per row/,
    );
  });

  it('refuses two sections of one name', () => {
    refused(`${HEAD}## Une fois\n\n## Une fois\n`, /two sections/);
  });
});

describe('the links of the pages', () => {
  const read = (pages) =>
    readPages(
      Object.keys(pages),
      (id) => `${HEAD}${pages[id]}`,
      (id) => `/docs/${id}`,
    );
  const to = (markdown) => read({ a: markdown, b: '## Clé maître\n' })[0].body[0].text[0].to;

  it('lead to a page, or to one of its sections under the name GitHub gives it', () => {
    assert.equal(to('[b](b.md)\n'), '/docs/b');
    assert.equal(to('[b](b.md#clé-maître)\n'), '/docs/b#cle-maitre');
  });

  it('lead to a section of the same page', () => {
    assert.equal(read({ a: '[plus bas](#ici)\n\n## Ici\n' })[0].body[0].text[0].to, '/docs/a#ici');
  });

  it('stay on the site for its own address, and leave it for another', () => {
    assert.equal(to('[fichiers](https://devnotes.fr/telecharger)\n'), '/telecharger');
    assert.equal(to('[fichiers](https://github.com/x)\n'), 'https://github.com/x');
  });

  it('are refused when they lead nowhere', () => {
    assert.throws(() => to('[c](c.md)\n'), { message: /there is no page c/ });
    assert.throws(() => to('[b](b.md#rien)\n'), { message: /b has no such section/ });
    assert.throws(() => to('[b](../b.txt)\n'), { message: /leads nowhere/ });
  });
});

it('a section is called as GitHub calls it, without accents or spare dashes', () => {
  assert.equal(slug('Où vont mes notes ?'), 'ou-vont-mes-notes');
  assert.equal(slug('où-vont-mes-notes-'), 'ou-vont-mes-notes');
  assert.equal(slug('L’API : `GET`, « v1 »'), 'lapi-get-v1');
});
