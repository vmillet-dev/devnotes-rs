export const MARKDOWN_GROUPS = ['headings', 'emphasis', 'lists', 'links', 'code', 'blocks', 'raw'] as const;
export type MarkdownGroup = (typeof MARKDOWN_GROUPS)[number];

export interface MarkdownEntry {
  /** As it is written, and what the row copies; `␣` stands for a space that matters. */
  readonly syntax: string;
  readonly group: MarkdownGroup;
  readonly commonmark: boolean;
  /** GitHub Flavored Markdown, and what github.com adds to it: footnotes, emoji. */
  readonly github: boolean;
  readonly example: string;
}

export interface MarkdownWords {
  readonly entries: Readonly<Record<string, string>>;
}

const BOTH = { commonmark: true, github: true } as const;
const GITHUB = { commonmark: false, github: true } as const;

export const MARKDOWN_ENTRIES: readonly MarkdownEntry[] = [
  { syntax: '# …', group: 'headings', ...BOTH, example: '## Installation' },
  { syntax: '…\n===', group: 'headings', ...BOTH, example: 'Installation\n============' },

  { syntax: '**…**', group: 'emphasis', ...BOTH, example: '**important**' },
  { syntax: '__…__', group: 'emphasis', ...BOTH, example: '__important__' },
  { syntax: '*…*', group: 'emphasis', ...BOTH, example: '*nuance*' },
  { syntax: '_…_', group: 'emphasis', ...BOTH, example: '_nuance_' },
  { syntax: '***…***', group: 'emphasis', ...BOTH, example: '***crucial***' },
  { syntax: '~~…~~', group: 'emphasis', ...GITHUB, example: '~~obsolète~~' },

  { syntax: '- …', group: 'lists', ...BOTH, example: '- un\n- deux' },
  { syntax: '1. …', group: 'lists', ...BOTH, example: '1. premier\n2. second' },
  { syntax: '␣␣- …', group: 'lists', ...BOTH, example: '- un\n  - sous-élément' },
  { syntax: '- [ ] …', group: 'lists', ...GITHUB, example: '- [ ] à faire\n- [x] fait' },

  { syntax: '[…](url)', group: 'links', ...BOTH, example: '[la doc](https://exemple.fr)' },
  { syntax: '[…](url "…")', group: 'links', ...BOTH, example: '[la doc](https://exemple.fr "Guide")' },
  { syntax: '<url>', group: 'links', ...BOTH, example: '<https://exemple.fr>' },
  { syntax: 'https://…', group: 'links', ...GITHUB, example: 'Voir https://exemple.fr' },
  { syntax: '[…][ref]', group: 'links', ...BOTH, example: '[la doc][1]\n\n[1]: https://exemple.fr' },
  { syntax: '![…](url)', group: 'links', ...BOTH, example: '![logo](logo.png)' },

  { syntax: '`…`', group: 'code', ...BOTH, example: '`npm install`' },
  { syntax: '```lang', group: 'code', ...BOTH, example: '```js\nconst port = 8080;\n```' },
  { syntax: '␣␣␣␣…', group: 'code', ...BOTH, example: '    code indenté' },

  { syntax: '> …', group: 'blocks', ...BOTH, example: '> citation\n>\n> > imbriquée' },
  { syntax: '---', group: 'blocks', ...BOTH, example: 'au-dessus\n\n---\n\nen dessous' },
  { syntax: '| … |', group: 'blocks', ...GITHUB, example: '| Nom | Port |\n| --- | --- |\n| api | 8080 |' },
  {
    syntax: '| :-- | :-: | --: |',
    group: 'blocks',
    ...GITHUB,
    example: '| Gauche | Centre | Droite |\n| :-- | :-: | --: |',
  },

  { syntax: '…␣␣', group: 'raw', ...BOTH, example: 'ligne  \nsuivante' },
  { syntax: '…\\', group: 'raw', ...BOTH, example: 'ligne\\\nsuivante' },
  { syntax: '\\*', group: 'raw', ...BOTH, example: '\\*pas en italique\\*' },
  { syntax: '<tag>', group: 'raw', ...BOTH, example: '<kbd>Ctrl</kbd>+<kbd>C</kbd>' },
  { syntax: '<!-- … -->', group: 'raw', ...BOTH, example: '<!-- jamais affiché -->' },
  { syntax: '[^1]', group: 'raw', ...GITHUB, example: 'Texte[^1]\n\n[^1]: La note.' },
  { syntax: ':emoji:', group: 'raw', ...GITHUB, example: 'Bravo :tada:' },
];
