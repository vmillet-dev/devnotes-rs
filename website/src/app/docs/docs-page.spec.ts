import { describe, expect, it } from 'vitest';

import { renderPage } from '../../testing/render-page';
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

describe('the documentation', () => {
  it('has a text for every page of the index, and no page outside it', () => {
    expect(DOC_PAGES.map(({ slug }) => slug).sort()).toEqual(DOC_INDEX.map(({ slug }) => slug).sort());
  });

  it('links only to pages that exist', () => {
    const paths = new Set(['/', ...DOC_INDEX.map(({ slug }) => docPath(slug))]);
    for (const page of DOC_PAGES) {
      for (const inline of page.body.flatMap(inlines)) {
        if (typeof inline === 'object' && 'link' in inline && inline.to.startsWith('/')) {
          expect(paths, `${page.slug}: ${inline.to}`).toContain(inline.to);
        }
      }
    }
  });

  it('gives every heading of a page its own anchor', () => {
    for (const page of DOC_PAGES) {
      const ids = page.body.flatMap((block) =>
        block.kind === 'h2' || block.kind === 'h3' ? [block.id] : [],
      );
      expect(new Set(ids).size, page.slug).toBe(ids.length);
    }
  });

  for (const [index, entry] of DOC_INDEX.entries()) {
    describe(docPath(entry.slug), () => {
      it('has one h1, its title, and the contents beside it match its sections', async () => {
        const page = await renderPage(DocsPage, { slug: entry.slug });
        const text = DOC_PAGES.find(({ slug }) => slug === entry.slug);

        expect(page.querySelectorAll('h1')).toHaveLength(1);
        expect(page.querySelector('h1')?.textContent?.trim()).toBe(text?.title);
        const sections = [...page.querySelectorAll('article h2[id]')].map((heading) => heading.id);
        const contents = [...page.querySelectorAll('nav.toc a')].map(
          (link) => link.getAttribute('href')?.split('#')[1],
        );
        expect(contents).toEqual(sections);
      });

      it('marks itself in the sidebar and chains to its neighbours', async () => {
        const page = await renderPage(DocsPage, { slug: entry.slug });

        expect(page.querySelector('.side [aria-current="page"]')?.getAttribute('href')).toBe(
          docPath(entry.slug),
        );
        const previous = DOC_INDEX[index - 1];
        const next = DOC_INDEX[index + 1];
        expect(page.querySelector('a[rel="prev"]')?.getAttribute('href') ?? null).toBe(
          previous ? docPath(previous.slug) : null,
        );
        expect(page.querySelector('a[rel="next"]')?.getAttribute('href') ?? null).toBe(
          next ? docPath(next.slug) : null,
        );
      });
    });
  }

  it('names every code block and makes it reachable by keyboard', async () => {
    const page = await renderPage(DocsPage, { slug: 'prise-en-main' });
    const blocks = [...page.querySelectorAll('pre')];

    expect(blocks.length).toBeGreaterThan(0);
    for (const block of blocks) {
      expect(block.getAttribute('tabindex')).toBe('0');
      expect(block.getAttribute('aria-label')).toMatch(/^Code : /);
    }
  });
});
