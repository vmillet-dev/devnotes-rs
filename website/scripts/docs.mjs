// Before `ng build`, `ng serve` and `ng test`: the pages of `docs/content`, read from Markdown into
// the blocks `DocBlocks` draws. Only what docs/content/README.md describes is read; anything else
// stops the build with its file and line, rather than reaching the page as a near miss.
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

import { Lexer } from 'marked';

import { workspaceVersion } from '../../scripts/version.mjs';

const CONTENT = fileURLToPath(new URL('../../docs/content', import.meta.url));
const OUT = fileURLToPath(new URL('../src/app/docs/generated/pages.ts', import.meta.url));
const CARGO = fileURLToPath(new URL('../../Cargo.toml', import.meta.url));
const LANGS = ['fr', 'en'];
/** A page of the site that is not documentation is named in full, and followed on the site itself. */
const SITE = 'https://devnotes.fr/';
const TONES = { '[!TIP]': 'tip', '[!WARNING]': 'warn' };
/** What heads a table of shortcuts, per language. */
const SHORTCUTS = ['Touches', 'Keys'];
/** The colours of a folder, as the tokens name them (`--green-rgb`). */
const COLOURS = ['green', 'blue', 'purple', 'teal', 'olive', 'pink', 'grey', 'amber'];
/** A link to a page of the documentation, with or without a section; a section alone is this page's. */
const PAGE_LINK = /^(?:([\w-]+)\.md)?(?:#(.+))?$/;

/** A heading's address: GitHub's, without accents or spare dashes; a link in GitHub's form leads there. */
export function slug(heading) {
  return heading
    .normalize('NFD')
    .replace(/\p{M}/gu, '')
    .toLowerCase()
    .replace(/[^\p{L}\p{N}\s_-]/gu, '')
    .replace(/[\s-]+/g, '-')
    .replace(/^-|-$/g, '');
}

/** Two texts in a row are one: an escaped character cuts a sentence that holds no mark. */
function joined(runs, run) {
  const last = runs.at(-1);
  return typeof run === 'string' && typeof last === 'string'
    ? [...runs.slice(0, -1), last + run]
    : [...runs, run];
}

/** `refuse(message)` throws, with the line; `follow(href)` is the address a link leads to. */
function reader(refuse, follow) {
  function text(value) {
    const stray = /\*|(?<!\w)_(?=\w)|&\w+;|<\/?[a-z]/i.exec(value);
    if (stray) refuse(`« ${stray[0]} » is left in « ${value.trim()} »: close the mark, or escape it`);
    return value;
  }

  function plain(tokens) {
    const said = tokens.map((token) => {
      if (token.type === 'escape') return token.text;
      if (token.type !== 'text') refuse(`plain text is expected, not ${token.raw}`);
      return text(token.text);
    });
    return said.join('');
  }

  function inline(tokens) {
    const runs = [];
    for (let at = 0; at < tokens.length; at += 1) {
      const token = tokens[at];
      switch (token.type) {
        case 'text':
          runs.push(text(token.text));
          break;
        case 'escape':
          runs.push(token.text);
          break;
        case 'codespan':
          runs.push({ code: token.text });
          break;
        case 'strong':
          runs.push({ strong: plain(token.tokens) });
          break;
        case 'em':
          runs.push({ ui: plain(token.tokens) });
          break;
        case 'link':
          runs.push({ link: plain(token.tokens), to: follow(token.href) });
          break;
        case 'html':
          if (token.raw !== '<kbd>' || tokens[at + 2]?.raw !== '</kbd>') refuse(`${token.raw} is not read`);
          runs.push({ keys: plain([tokens[at + 1]]).split(' ') });
          at += 2;
          break;
        default:
          refuse(`${token.type} is not read: ${token.raw}`);
      }
    }
    return runs.reduce(joined, []);
  }

  /** The inline tokens of what holds a single paragraph: an item, a note. */
  function only(tokens, what) {
    const [first, ...rest] = tokens.filter(({ type }) => type !== 'space');
    if (rest.length || !['paragraph', 'text'].includes(first?.type)) refuse(`${what} holds one paragraph`);
    return first.tokens ?? [];
  }

  function list(token) {
    const items = token.items.map((item) => {
      if (item.task) refuse('a task list is not read');
      return only(item.tokens, 'an item');
    });
    const pages = items.map((item) =>
      item.length === 1 && item[0].type === 'link' ? PAGE_LINK.exec(item[0].href) : null,
    );
    if (pages.every((page) => page?.[1] && !page[2]))
      return { kind: 'guides', pages: pages.map(([, id]) => id) };
    return { kind: 'list', ordered: token.ordered, items: items.map(inline) };
  }

  function note(token) {
    const [first, ...rest] = only(token.tokens, 'a note');
    const [mark, ...said] = (first?.type === 'text' ? first.text : '').split('\n');
    const tone = TONES[mark.trim()];
    if (!tone) refuse('a quote begins with [!TIP] or [!WARNING], alone on its line');
    const begin = said.join('\n');
    return { kind: 'note', tone, text: inline([...(begin ? [{ type: 'text', text: begin }] : []), ...rest]) };
  }

  function table(token, caption) {
    const head = token.header.map((cell) => plain(cell.tokens));
    const rows = token.rows.map((row) => row.map((cell) => inline(cell.tokens)));
    if (!SHORTCUTS.includes(head[0])) return { kind: 'table', caption, head, rows };

    const shortcuts = rows.map(([first], at) => {
      if (first.length !== 1 || !first[0].keys) refuse('a table of shortcuts holds one <kbd> per row');
      return { keys: first[0].keys, does: plain(token.rows[at][1].tokens) };
    });
    return { kind: 'keys', caption, rows: shortcuts };
  }

  /** A paragraph of bold alone, just above a table or a code block, is its title. */
  function titleOf(token) {
    const [only, ...rest] = token.type === 'paragraph' ? token.tokens : [];
    return only?.type === 'strong' && !rest.length ? plain(only.tokens) : null;
  }

  function block(token, caption) {
    switch (token.type) {
      case 'heading': {
        if (token.depth !== 2 && token.depth !== 3) refuse('a section is ## or ###');
        const said = plain(token.tokens);
        return { kind: `h${token.depth}`, id: slug(said), text: said };
      }
      case 'paragraph':
        return { kind: 'p', text: inline(token.tokens) };
      case 'list':
        return list(token);
      case 'blockquote':
        return note(token);
      case 'code':
        return { kind: 'code', title: caption, lang: token.lang ?? '', code: token.text };
      case 'table':
        return table(token, caption);
      default:
        return refuse(`${token.type} is not read: ${token.raw.trim().slice(0, 60)}`);
    }
  }

  return { block, plain, titleOf };
}

/** Reads a page. `follow(href, refuse)` is the address a link leads to; `name` is its file in a refusal. */
export function readPage(id, markdown, follow, name = `${id}.md`) {
  let line = 1;
  const refuse = (message) => {
    throw new Error(`${name}:${line}: ${message}`);
  };
  const { block, plain, titleOf } = reader(refuse, (href) => follow(href, refuse));

  const page = { id, title: null, summary: null, body: [] };
  let caption = null;
  const tokens = new Lexer({ gfm: true }).lex(markdown);
  const solid = tokens.filter(({ type }) => type !== 'space');
  for (const token of tokens) {
    const next = solid[solid.indexOf(token) + 1];
    if (token.type === 'space') {
      // Between two blocks.
    } else if (page.title === null) {
      if (token.type !== 'heading' || token.depth !== 1) refuse('a page begins with # and its title');
      page.title = plain(token.tokens);
    } else if (page.summary === null) {
      if (token.type !== 'paragraph') refuse('the title is followed by one paragraph, the summary');
      page.summary = plain(token.tokens);
    } else if (['table', 'code'].includes(next?.type) && titleOf(token) !== null) {
      caption = titleOf(token);
    } else {
      if (['table', 'code'].includes(token.type) && caption === null) {
        refuse(`a ${token.type === 'code' ? 'code block' : 'table'} has its title in bold on the line above`);
      }
      page.body.push(block(token, caption));
      caption = null;
    }
    line += token.raw.split('\n').length - 1;
  }
  if (page.summary === null) refuse('a page has a title and a summary');
  const ids = page.body.flatMap((each) => (each.id ? [each.id] : []));
  const twice = ids.find((each, at) => ids.indexOf(each) !== at);
  if (twice) refuse(`two sections are called ${twice}`);
  return page;
}

/** Reads every page of a language; `source(id)` is its Markdown and `address(id)` its path on the site. */
export function readPages(ids, source, address, name = (id) => `${id}.md`) {
  const links = [];
  const pages = ids.map((id) => {
    const follow = (href, refuse) => {
      if (href.startsWith(SITE)) return href.slice(SITE.length - 1);
      if (/^https?:\/\//.test(href)) return href;
      const [, to = id, section] = PAGE_LINK.exec(href) ?? refuse(`${href} leads nowhere`);
      if (!ids.includes(to)) refuse(`${href}: there is no page ${to}`);
      const anchor = section ? slug(decodeURIComponent(section)) : '';
      if (anchor) links.push({ to, anchor, refuse: () => refuse(`${href}: ${to} has no such section`) });
      return address(to) + (anchor && `#${anchor}`);
    };
    return readPage(id, source(id), follow, name(id));
  });
  for (const { to, anchor, refuse } of links) {
    const target = pages.find((page) => page.id === to);
    if (!target.body.some((block) => block.id === anchor)) refuse();
  }
  return pages;
}

function main() {
  const index = JSON.parse(readFileSync(join(CONTENT, 'index.json'), 'utf8'));
  const entries = index.groups.flatMap((group) => group.pages);
  const odd = entries.find(({ colour }) => !COLOURS.includes(colour));
  if (odd) throw new Error(`docs/content/index.json: ${odd.id} has no colour called ${odd.colour}`);
  const path = (id, lang) => {
    const own = entries.find((entry) => entry.id === id).path[lang];
    return (lang === 'fr' ? '/docs' : '/en/docs') + (own && `/${own}`);
  };
  const read = (lang) =>
    readPages(
      entries.map(({ id }) => id),
      (id) => readFileSync(join(CONTENT, lang, `${id}.md`), 'utf8'),
      (id) => path(id, lang),
      (id) => `docs/content/${lang}/${id}.md`,
    );
  const pages = Object.fromEntries(LANGS.map((lang) => [lang, read(lang)]));
  const version = workspaceVersion(readFileSync(CARGO, 'utf8'));
  if (!version) throw new Error('Cargo.toml: no version in [workspace.package]');

  mkdirSync(dirname(OUT), { recursive: true });
  writeFileSync(
    OUT,
    `// Written by scripts/docs.mjs from docs/content.
import { Lang, french } from '../../core/lang';
import { DocPage } from '../doc.model';

const PAGES: Readonly<Record<Lang, readonly DocPage[]>> = ${JSON.stringify(pages)};

export const DOC_PAGES: Readonly<Record<Lang, readonly DocPage[]>> = { fr: french(PAGES.fr), en: PAGES.en };

/** The version the pages describe, to the minor: the workspace's, in Cargo.toml. */
export const CHECKED_ON = ${JSON.stringify(version.split('.').slice(0, 2).join('.'))};
`,
  );
  console.log(`${entries.length} pages read from docs/content, in ${LANGS.join(' and ')}.`);
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? '').href) {
  try {
    main();
  } catch (error) {
    console.error(error.message);
    process.exit(1);
  }
}
