import { describe, expect, it } from 'vitest';

import { accessibleName, renderPage } from '../../testing/render-page';
import { HomePage } from './home-page';

describe('HomePage', () => {
  it('has one h1, then a h2 for every section the progress line follows', async () => {
    const page = await renderPage(HomePage);

    expect(page.querySelectorAll('h1')).toHaveLength(1);
    const sections = [...page.querySelectorAll('section[id]')].map((section) => section.id);
    expect(sections).toEqual([
      'snippets',
      'ranger',
      'code',
      'outils',
      'client-api',
      'chiffrement',
      'et-aussi',
    ]);
    for (const id of sections) {
      const section = page.querySelector(`section#${id}`);
      expect(section?.querySelector('h2')).not.toBeNull();
      expect(section?.getAttribute('aria-labelledby')).toBe(section?.querySelector('h2')?.id);
    }
  });

  it('shows every feature’s steps as buttons a visitor can press', async () => {
    const page = await renderPage(HomePage);
    const steps = [...page.querySelectorAll('.steps button')];

    expect(steps).toHaveLength(18);
    for (const step of steps) expect(accessibleName(step)).not.toBe('');
  });

  it('keeps the scenes out of the accessibility tree, and the board described', async () => {
    const page = await renderPage(HomePage);

    for (const stage of page.querySelectorAll('.story-stage'))
      expect(stage.getAttribute('aria-hidden')).toBe('true');
    expect(page.querySelector('[role="img"]')?.getAttribute('aria-label')).toContain('tableau');
  });
});
