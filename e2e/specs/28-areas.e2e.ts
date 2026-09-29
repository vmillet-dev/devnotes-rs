import { $, expect } from '@wdio/globals';

import { canvas } from '../pageobjects/canvas.page.js';
import { editor } from '../pageobjects/editor.page.js';
import { rail } from '../pageobjects/sidebar.page.js';
import { emitGlobalAction, press, reloadCanvas, testid } from '../support/app.js';

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
    await option('tools-rail', 'notes').click();
    await showsNotes();

    await rail.show();
    expect(await inTitlebar('tools').isExisting()).toBe(false);
  });
});
