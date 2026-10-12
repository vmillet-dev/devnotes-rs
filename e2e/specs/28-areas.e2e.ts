import { $, browser, expect } from '@wdio/globals';

import { canvas } from '../pageobjects/canvas.page.js';
import { editor } from '../pageobjects/editor.page.js';
import { rail } from '../pageobjects/sidebar.page.js';
import { emitGlobalAction, eventually, press, reloadCanvas, testid } from '../support/app.js';

/**
 * The notes page stays in the DOM behind another area, switch included: every selector
 * here names the rail or the bar it reads.
 */
const option = (container: string, area: string) =>
  $(`${testid(container)} ${testid('area-option')}[data-area="${area}"]`);
const inTitlebar = (area: string) => $(`app-titlebar ${testid('area-option')}[data-area="${area}"]`);

async function showsTools(): Promise<void> {
  await $(testid('tools-page')).waitForDisplayed({ timeout: 10_000 });
  expect(await $(testid('canvas')).isDisplayed()).toBe(false);
}

async function showsNotes(): Promise<void> {
  await $(testid('tools-page')).waitForExist({ reverse: true, timeout: 10_000 });
  await $(testid('canvas')).waitForDisplayed({ timeout: 10_000 });
}

/** Measured, not read off the preference: the rail has to actually be that wide. */
const railWidth = (rail: string): Promise<number> =>
  browser.execute(
    (selector: string) => Math.round(document.querySelector(selector)?.getBoundingClientRect().width ?? 0),
    testid(rail),
  );

/** The edge's keyboard twin. Focused from script: its own `pointerdown` starts a drag instead. */
async function nudge(edge: string, key: 'ArrowLeft' | 'ArrowRight', times: number): Promise<void> {
  await browser.execute(
    (selector: string) => (document.querySelector(selector) as HTMLElement | null)?.focus(),
    testid(edge),
  );
  await browser.keys(Array.from({ length: times }, () => key));
}

describe('Switching between areas', () => {
  before(async () => {
    await canvas.open();
    await rail.show();
  });

  after(async () => {
    await press('1', ['Control']);
    await rail.show();
    await canvas.clearSearch();
  });

  it('goes to the tools from the head of the rail, and back', async () => {
    await option('library-rail', 'tools').click();
    await showsTools();
    expect(await option('tools-rail', 'tools').getAttribute('aria-current')).toBe('page');

    await option('tools-rail', 'notes').click();
    await showsNotes();
  });

  it('keeps the notes as they were across the trip', async () => {
    await canvas.search('zzz-nothing-matches');

    await press('2', ['Control']);
    await showsTools();
    await press('1', ['Control']);
    await showsNotes();

    expect(await canvas.searchQuery()).toBe('zzz-nothing-matches');
    await canvas.clearSearch();
  });

  it('reopens on the area it was left in', async () => {
    await press('2', ['Control']);
    await showsTools();

    await reloadCanvas();

    await showsTools();
    await press('1', ['Control']);
    await showsNotes();
  });

  it('comes back to the notes for what the native side asks', async () => {
    await press('2', ['Control']);
    await showsTools();

    await emitGlobalAction('new-note');

    await editor.waitOpen();
    await showsNotes();
    await press('Escape');
    await $(testid('editor-title')).waitForExist({ reverse: true, timeout: 10_000 });
  });

  it('moves to the titlebar while the rail is hidden', async () => {
    await rail.hide();

    await inTitlebar('tools').click();
    await showsTools();
    await inTitlebar('notes').click();
    await showsNotes();

    await rail.show();
    expect(await inTitlebar('tools').isExisting()).toBe(false);
  });

  /**
   * One state for the three areas. Hidden in the notes, the rail used to come back in the
   * tools, and to stay hidden in HTTP with nothing on screen to show it again.
   */
  it('hides the rail in the three areas at once, and shows it again from any of them', async () => {
    await rail.hide();

    await press('2', ['Control']);
    await showsTools();
    expect(await $(testid('tools-rail')).isExisting()).toBe(false);
    expect(await inTitlebar('http').isDisplayed()).toBe(true);
    expect(await $(testid('tools-rail-toggle')).getAttribute('aria-pressed')).toBe('false');

    await press('3', ['Control']);
    await $(testid('http-page')).waitForDisplayed({ timeout: 10_000 });
    expect(await $(testid('http-rail')).isExisting()).toBe(false);

    await rail.toggleIn('http');
    await $(testid('http-rail')).waitForDisplayed({ timeout: 10_000 });
    expect(await inTitlebar('tools').isExisting()).toBe(false);

    await press('2', ['Control']);
    await showsTools();
    await $(testid('tools-rail')).waitForDisplayed({ timeout: 10_000 });

    await press('1', ['Control']);
    await showsNotes();
    expect(await rail.isShowing()).toBe(true);
  });

  /**
   * One width for the three rails, and an edge on each. HTTP's used to take 240px whatever
   * was stored, so the rail jumped on the way in from the notes and back on the way out.
   */
  it('keeps the rail as wide from one area to the next, and lets each of them resize it', async () => {
    await rail.narrow();
    const floor = await eventually(
      () => rail.width(),
      (width) => width === 160,
      'the rail at its floor',
    );

    await press('3', ['Control']);
    await $(testid('http-rail')).waitForDisplayed({ timeout: 10_000 });
    expect(await railWidth('http-rail')).toBe(floor);

    await nudge('http-rail-edge', 'ArrowRight', 5);
    const widened = await eventually(
      () => railWidth('http-rail'),
      (width) => width === floor + 80,
      'the HTTP rail widened from its own edge',
    );

    await press('2', ['Control']);
    await showsTools();
    expect(await railWidth('tools-rail')).toBe(widened);
    await nudge('tools-rail-edge', 'ArrowLeft', 2);
    await eventually(
      () => railWidth('tools-rail'),
      (width) => width === widened - 32,
      'the tools rail narrowed',
    );

    await press('1', ['Control']);
    await showsNotes();
    expect(await rail.width()).toBe(widened - 32);
    await rail.narrow();
    await eventually(
      () => rail.width(),
      (width) => width === floor,
      'the rail back at its floor',
    );
  });

  /** One key for the three: the canvas had it, HTTP a copy of it, the tools none. */
  it('hides and shows the rail on Ctrl+B from the tools, a field focused or not', async () => {
    await press('2', ['Control']);
    await showsTools();
    await $(testid('tools-rail')).waitForDisplayed({ timeout: 10_000 });

    await $(testid('tools-search')).click();
    await press('b', ['Control']);
    await $(testid('tools-rail')).waitForExist({ reverse: true, timeout: 10_000 });

    await press('b', ['Control']);
    await $(testid('tools-rail')).waitForDisplayed({ timeout: 10_000 });

    await press('1', ['Control']);
    await showsNotes();
  });
});
