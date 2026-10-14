import { Block, DocPage, Inline, Text } from './doc.model';

/** One section of a page: what the search reads, and where a result leads. */
export interface SearchEntry {
  readonly slug: string;
  readonly page: string;
  /** The section's heading id, empty for the top of the page. */
  readonly anchor: string;
  readonly heading: string;
  readonly text: string;
  readonly folded: string;
}

export interface SearchResult {
  readonly entry: SearchEntry;
  readonly before: string;
  readonly match: string;
  readonly after: string;
}

/**
 * Lower case, accents dropped, one character for one: the folded text keeps the original's
 * positions, so a match found in one is cut out of the other. As the application's search.
 */
export function fold(text: string): string {
  return [...text]
    .map((char) => char.normalize('NFD').replace(/\p{M}/gu, '').toLowerCase()[0] ?? char)
    .join('');
}

function words(inline: Inline): string {
  if (typeof inline === 'string') return inline;
  if ('code' in inline) return inline.code;
  if ('keys' in inline) return inline.keys.join(' ');
  if ('strong' in inline) return inline.strong;
  if ('ui' in inline) return inline.ui;
  return inline.link;
}

const say = (text: Text) => text.map(words).join('');

function blockText(block: Block): string {
  switch (block.kind) {
    case 'p':
    case 'note':
      return say(block.text);
    case 'list':
      return block.items.map(say).join(' ');
    case 'table':
      return [block.caption, ...block.rows.map((row) => row.map(say).join(' '))].join(' ');
    case 'keys':
      return [block.caption, ...block.rows.map((row) => `${row.keys.join(' ')} ${row.does}`)].join(' ');
    case 'code':
      return `${block.title} ${block.code}`;
    case 'h2':
    case 'h3':
      return block.text;
    case 'guides':
      return '';
  }
}

/** Every page cut at its `h2`s: a result points at the section that holds the words. */
export function searchIndex(pages: readonly DocPage[]): readonly SearchEntry[] {
  return pages.flatMap((page) => {
    const entries: { anchor: string; heading: string; parts: string[] }[] = [
      { anchor: '', heading: page.title, parts: [page.summary] },
    ];
    for (const block of page.body) {
      if (block.kind === 'h2') entries.push({ anchor: block.id, heading: block.text, parts: [] });
      else entries.at(-1)?.parts.push(blockText(block));
    }
    return entries.map(({ anchor, heading, parts }) => {
      const text = parts.join(' ').replace(/\s+/g, ' ').trim();
      return { slug: page.slug, page: page.title, anchor, heading, text, folded: fold(`${heading} ${text}`) };
    });
  });
}

const EXCERPT = 70;

/** Every word of the query, in any order; the heading's own words count most. */
export function search(index: readonly SearchEntry[], query: string, limit = 8): readonly SearchResult[] {
  const terms = fold(query).split(/\s+/).filter(Boolean);
  if (!terms.length) return [];
  const [first] = terms;

  return index
    .filter((entry) => terms.every((term) => entry.folded.includes(term)))
    .map((entry) => ({ entry, score: terms.filter((term) => fold(entry.heading).includes(term)).length }))
    .sort((a, b) => b.score - a.score)
    .slice(0, limit)
    .map(({ entry }) => {
      const at = fold(entry.text).indexOf(first ?? '');
      if (at < 0) return { entry, before: entry.text.slice(0, EXCERPT * 2), match: '', after: '' };
      const end = at + (first?.length ?? 0);
      const start = Math.max(0, at - EXCERPT);
      return {
        entry,
        before: `${start > 0 ? '…' : ''}${entry.text.slice(start, at)}`,
        match: entry.text.slice(at, end),
        after: `${entry.text.slice(end, end + EXCERPT)}${end + EXCERPT < entry.text.length ? '…' : ''}`,
      };
    });
}
