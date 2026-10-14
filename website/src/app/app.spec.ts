import { describe, expect, it } from 'vitest';

import { accessibleName, renderPage } from '../testing/render-page';
import { App } from './app';

describe('App', () => {
  it('lays the page out in landmarks, with a way past the header', async () => {
    const page = await renderPage(App);

    expect(page.querySelector('header')).not.toBeNull();
    expect(page.querySelector('main#contenu')).not.toBeNull();
    expect(page.querySelector('footer')).not.toBeNull();
    expect(page.querySelector('nav[aria-label="Principale"]')).not.toBeNull();
    expect(page.querySelector('.skip-link')?.getAttribute('href')).toBe('/#contenu');
  });

  it('names every control of the header', async () => {
    const page = await renderPage(App);
    const buttons = [...page.querySelectorAll('header button')];

    expect(buttons.length).toBeGreaterThan(0);
    for (const button of buttons) expect(accessibleName(button)).not.toBe('');
    expect(page.querySelector('.menu-btn')?.getAttribute('aria-expanded')).toBe('false');
  });

  it('links the documentation and the download from the header', async () => {
    const page = await renderPage(App);
    const links = [...page.querySelectorAll<HTMLAnchorElement>('nav[aria-label="Principale"] a')];

    expect(links.map((link) => link.getAttribute('href'))).toEqual(['/#snippets', '/docs', '/#telecharger']);
  });
});
