import { describe, expect, it } from 'vitest';

import { renderPage } from '../../testing/render-page';
import { DOC_INDEX, docPath } from './doc-index';
import { DocsPage } from './docs-page';
import { DOC_PAGES } from './generated/pages';

describe('the documentation', () => {
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
