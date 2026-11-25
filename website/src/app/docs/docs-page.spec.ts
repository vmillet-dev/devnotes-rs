import { describe, expect, it } from 'vitest';

import { renderPage } from '../../testing/render-page';
import { Lang, downloadPath, homePath } from '../core/lang';
import { DOC_PAGES } from './content/pages';
import { Block, Inline } from './doc.model';
import { DOC_INDEX, docPath } from './doc-index';
import { DocsPage } from './docs-page';

function inlines(block: Block): readonly Inline[] {
  switch (block.kind) {
    case 'p':
    case 'note':
      return block.text;
    case 'list':
      return block.items.flat();
    case 'table':
      return block.rows.flat(2);
    case 'h2':
    case 'h3':
    case 'code':
    case 'keys':
    case 'guides':
      return [];
  }
}

const LANGS: readonly Lang[] = ['fr', 'en'];

describe('the documentation', () => {
  for (const lang of LANGS) {
    describe(lang, () => {
      it('has a text for every page of the index, in its order', () => {
        expect(DOC_PAGES[lang].map(({ id }) => id)).toEqual(DOC_INDEX.map(({ id }) => id));
      });

      it('links only to pages that exist in its own language', () => {
        const paths = new Set([
          homePath(lang),
          downloadPath(lang),
          ...DOC_INDEX.map(({ id }) => docPath(id, lang)),
        ]);
        for (const page of DOC_PAGES[lang]) {
          for (const inline of page.body.flatMap(inlines)) {
            if (typeof inline === 'object' && 'link' in inline && inline.to.startsWith('/')) {
              expect(paths, `${page.id}: ${inline.to}`).toContain(inline.to);
            }
          }
        }
      });

      it('gives every heading of a page its own anchor', () => {
        for (const page of DOC_PAGES[lang]) {
          const ids = page.body.flatMap((block) =>
            block.kind === 'h2' || block.kind === 'h3' ? [block.id] : [],
          );
          expect(new Set(ids).size, page.id).toBe(ids.length);
        }
      });
    });
  }

  for (const [index, entry] of DOC_INDEX.entries()) {
    describe(docPath(entry.id, 'fr'), () => {
      it('has one h1, its title, and the contents beside it match its sections', async () => {
        const page = await renderPage(DocsPage, { doc: entry.id });
        const text = DOC_PAGES.fr.find(({ id }) => id === entry.id);

        expect(page.querySelectorAll('h1')).toHaveLength(1);
        expect(page.querySelector('h1')?.textContent?.trim()).toBe(text?.title);
        const sections = [...page.querySelectorAll('article h2[id]')].map((heading) => heading.id);
        const contents = [...page.querySelectorAll('nav.toc a')].map(
          (link) => link.getAttribute('href')?.split('#')[1],
        );
        expect(contents).toEqual(sections);
      });

      it('marks itself in the sidebar and chains to its neighbours', async () => {
        const page = await renderPage(DocsPage, { doc: entry.id });

        expect(page.querySelector('.side [aria-current="page"]')?.getAttribute('href')).toBe(
          docPath(entry.id, 'fr'),
        );
        const previous = DOC_INDEX[index - 1];
        const next = DOC_INDEX[index + 1];
        expect(page.querySelector('a[rel="prev"]')?.getAttribute('href') ?? null).toBe(
          previous ? docPath(previous.id, 'fr') : null,
        );
        expect(page.querySelector('a[rel="next"]')?.getAttribute('href') ?? null).toBe(
          next ? docPath(next.id, 'fr') : null,
        );
      });
    });
  }

  it('names every code block and makes it reachable by keyboard', async () => {
    const page = await renderPage(DocsPage, { doc: 'start' });
    const blocks = [...page.querySelectorAll('pre')];

    expect(blocks.length).toBeGreaterThan(0);
    for (const block of blocks) {
      expect(block.getAttribute('tabindex')).toBe('0');
      expect(block.getAttribute('aria-label')).toMatch(/^Code : /);
    }
  });
});
