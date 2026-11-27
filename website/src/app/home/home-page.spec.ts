import { describe, expect, it } from 'vitest';

import { renderPage } from '../../testing/render-page';
import { DAY_TEXT } from './day/day-text';
import { SCENE_WORDS } from './day/scene-words';
import { HomePage } from './home-page';

/** Every key of a dictionary, nested ones as `a.b`, and how many items each list holds. */
function shape(value: unknown, path = ''): string[] {
  if (Array.isArray(value))
    return [`${path}[${value.length}]`, ...value.flatMap((item, i) => shape(item, `${path}.${i}`))];
  if (value && typeof value === 'object')
    return Object.entries(value).flatMap(([key, item]) => shape(item, path ? `${path}.${key}` : key));
  return [path];
}

describe('HomePage', () => {
  it('has one h1, then three sections, each named by its own heading', async () => {
    const page = await renderPage(HomePage);

    expect(page.querySelectorAll('h1')).toHaveLength(1);
    const sections = [...page.querySelectorAll('section[id]')];
    expect(sections.map((section) => section.id)).toEqual(['app', 'journee', 'telecharger']);
    for (const section of sections) {
      const heading = section.querySelector(`#${section.getAttribute('aria-labelledby')}`);
      expect(heading?.tagName).toMatch(/^H[12]$/);
      expect(heading?.textContent?.trim()).not.toBe('');
    }
  });

  it('shows the application itself, described and at its own size, its six marks named by the legend', async () => {
    const page = await renderPage(HomePage);
    const capture = page.querySelector<HTMLImageElement>('.shot img');

    expect(capture?.getAttribute('alt')).toMatch(/^DevNotes\u00a0: /);
    expect(capture?.getAttribute('width')).toBe('2168');
    expect(capture?.getAttribute('height')).toBe('1362');
    expect(capture?.getAttribute('srcset')).toContain('/shots/notes-fr-dark-1084.webp 1084w');
    expect(page.querySelector('.shot source')?.getAttribute('srcset')).toContain(
      '/shots/notes-fr-light-2168.webp',
    );
    const marks = [...page.querySelectorAll('.shot .mark')];
    expect(marks).toHaveLength(6);
    for (const mark of marks) expect(mark.getAttribute('aria-hidden')).toBe('true');
    expect(page.querySelectorAll('.legend li')).toHaveLength(6);
  });

  it('sets its French as French is printed: no line break before : ; ? ! », nor after «', async () => {
    const page = await renderPage(HomePage);
    const walker = page.ownerDocument.createTreeWalker(page, NodeFilter.SHOW_TEXT);
    const loose: string[] = [];
    // The scenes draw the application, whose own words are typed with plain spaces.
    for (let node = walker.nextNode(); node; node = walker.nextNode()) {
      const text = node.textContent ?? '';
      if (/ [:;?!»]|« /.test(text) && !node.parentElement?.closest('code, kbd, app-day-scene'))
        loose.push(text);
    }

    expect(loose).toEqual([]);
  });

  it('tells the day in ten moments, each row named by its h3', async () => {
    const page = await renderPage(HomePage);
    const moments = [...page.querySelectorAll('article.mo')];

    expect(moments).toHaveLength(10);
    for (const moment of moments) {
      const heading = moment.querySelector('h3');
      expect(heading?.id).toBe(moment.getAttribute('aria-labelledby'));
      expect(moment.querySelector('time')?.textContent).toMatch(/^\d\d:\d\d$/);
    }
  });

  it('keeps the scenes out of the accessibility tree, and shows them finished without a script', async () => {
    const page = await renderPage(HomePage);
    const scenes = [...page.querySelectorAll('app-day-scene')];

    expect(scenes).toHaveLength(10);
    for (const scene of scenes) {
      expect(scene.getAttribute('aria-hidden')).toBe('true');
      expect(scene.classList).toContain('scene');
      expect(scene.classList).not.toContain('anim');
    }
    expect(page.querySelector('.rail')?.getAttribute('aria-hidden')).toBe('true');
  });

  it('offers every file of the release, each with its size', async () => {
    const page = await renderPage(HomePage);
    const files = [...page.querySelectorAll<HTMLAnchorElement>('.os a[href*="/releases/download/"]')];

    expect(files.length).toBeGreaterThanOrEqual(6);
    for (const file of files) expect(file.querySelector('span:last-child')?.textContent).toMatch(/\d Mo$/);
  });

  it('tells the same day in both languages, scene for scene', () => {
    const outline = (lang: 'fr' | 'en') =>
      DAY_TEXT[lang].moments.map(({ id, time, scene, tone }) => [id, time, scene, tone]);

    expect(outline('en')).toEqual(outline('fr'));
    expect(shape(SCENE_WORDS.en)).toEqual(shape(SCENE_WORDS.fr));
  });
});
