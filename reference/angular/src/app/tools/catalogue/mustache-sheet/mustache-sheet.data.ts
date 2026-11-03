export const MUSTACHE_GROUPS = ['values', 'sections', 'more'] as const;
export type MustacheGroup = (typeof MUSTACHE_GROUPS)[number];

export interface MustacheEntry {
  /** As it is written, and what the row copies. */
  readonly syntax: string;
  readonly group: MustacheGroup;
  readonly template: string;
  /** What the template is rendered with: its view as JSON, a partial. */
  readonly view: string;
  readonly output: string;
}

export interface MustacheWords {
  readonly entries: Readonly<Record<string, string>>;
}

export const MUSTACHE_ENTRIES: readonly MustacheEntry[] = [
  {
    syntax: '{{name}}',
    group: 'values',
    template: 'Bonjour {{name}}',
    view: '{ "name": "Ada" }',
    output: 'Bonjour Ada',
  },
  {
    syntax: '{{{name}}}',
    group: 'values',
    template: '{{{html}}}',
    view: '{ "html": "<b>Ada</b>" }',
    output: '<b>Ada</b>',
  },
  {
    syntax: '{{& name}}',
    group: 'values',
    template: '{{& html}}',
    view: '{ "html": "<b>Ada</b>" }',
    output: '<b>Ada</b>',
  },
  {
    syntax: '{{a.b}}',
    group: 'values',
    template: '{{user.name}}',
    view: '{ "user": { "name": "Ada" } }',
    output: 'Ada',
  },
  {
    syntax: '{{missing}}',
    group: 'values',
    template: '[{{missing}}]',
    view: '{}',
    output: '[]',
  },

  {
    syntax: '{{#list}}…{{/list}}',
    group: 'sections',
    template: '{{#items}}\n- {{label}}\n{{/items}}',
    view: '{ "items": [{ "label": "un" }, { "label": "deux" }] }',
    output: '- un\n- deux',
  },
  {
    syntax: '{{#flag}}…{{/flag}}',
    group: 'sections',
    template: '{{#admin}}Accès complet{{/admin}}',
    view: '{ "admin": true }',
    output: 'Accès complet',
  },
  {
    syntax: '{{#object}}…{{/object}}',
    group: 'sections',
    template: '{{#user}}{{name}} ({{role}}){{/user}}',
    view: '{ "user": { "name": "Ada", "role": "admin" } }',
    output: 'Ada (admin)',
  },
  {
    syntax: '{{^list}}…{{/list}}',
    group: 'sections',
    template: '{{^items}}Aucun élément{{/items}}',
    view: '{ "items": [] }',
    output: 'Aucun élément',
  },
  {
    syntax: '{{.}}',
    group: 'sections',
    template: '{{#tags}}#{{.}} {{/tags}}',
    view: '{ "tags": ["api", "web"] }',
    output: '#api #web',
  },

  {
    syntax: '{{! … }}',
    group: 'more',
    template: 'a{{! jamais rendu }}b',
    view: '{}',
    output: 'ab',
  },
  {
    syntax: '{{> partial}}',
    group: 'more',
    template: '{{> header}}',
    view: '{ "title": "Docs" }, header = "# {{title}}"',
    output: '# Docs',
  },
  {
    syntax: '{{=<% %>=}}',
    group: 'more',
    template: '{{=<% %>=}}<% name %>',
    view: '{ "name": "Ada" }',
    output: 'Ada',
  },
];
