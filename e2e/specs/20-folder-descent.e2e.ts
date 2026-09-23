import { browser, expect } from '@wdio/globals';

import { canvas } from '../pageobjects/canvas.page.js';
import { editor } from '../pageobjects/editor.page.js';
import { board } from '../pageobjects/board.page.js';
import { crumb } from '../pageobjects/header.page.js';
import { spaces } from '../pageobjects/sidebar.page.js';
import { blurField, eventually, press, reloadCanvas, testid, waitForCanvas } from '../support/app.js';
import { bridge, draft, homeSpaceId, query } from '../support/bridge.js';

/**
 * The descent: a space shows its folders, a folder shows its notes.
 *
 * ⚠️ A space of its own, like the two board files before it: nineteen files run first and
 * leave their notes in the home space.
 */
describe('Opening a folder', () => {
  let homeId = '';
  let spaceId = '';
  let perfId = '';

  async function inSpace(): Promise<void> {
    await reloadCanvas();
    await spaces.open();
    await spaces.option(spaceId).click();
    await waitForCanvas();
  }

  before(async () => {
    await canvas.open();
    homeId = await homeSpaceId();
    spaceId = (await bridge.createSpace({ name: 'Descente' })).id;

    perfId = (await bridge.createFolder({ spaceId, name: 'Perf' })).id;
    const inside = await bridge.createNote(draft({ spaceId, title: 'EXPLAIN lent sur join' }));
    await bridge.fileNotes([inside.id], perfId);
    await bridge.createNote(draft({ spaceId, title: 'Dump nocturne' }));

    await inSpace();
  });

  after(async () => {
    await bridge.deleteSpace(spaceId, homeId);
    await reloadCanvas();
    await spaces.open();
    await spaces.allOption().click();
    await reloadCanvas();
  });

  it('opens from the zone title on the board', async () => {
    await board.show('board');
    await board.openZone('Perf');
    await waitForCanvas();

    expect(await crumb.isShowing()).toBe(true);
    expect(await crumb.name()).toBe('Perf');
  });

  /** ⚠️ The inside of a folder is not spatial: no zones, no coordinates, nothing to draw. */
  it('shows an ordinary grid of its notes and nothing else', async () => {
    expect(await board.isShowing()).toBe(false);
    expect(await canvas.titles()).toEqual(['EXPLAIN lent sur join']);
  });

  /** There is one place to go from here, so the switchers give way to the breadcrumb. */
  it('puts the space switcher and the view switch away while it is open', async () => {
    expect(await browser.$(testid('space-switcher')).isExisting()).toBe(false);
    expect(await browser.$(testid('view-board')).isExisting()).toBe(false);
    expect(await browser.$(testid('folder-switcher')).isExisting()).toBe(false);
  });

  it('carries the folder colour on the breadcrumb', async () => {
    expect(await crumb.swatchClass()).toContain('is-blue');
  });

  /**
   * ⚠️ The flat view a folder produces had `show_create_ghost: false` like any other, so
   * the one place where creating a note files it had nothing to create from.
   */
  it('keeps the slot a note is created from, and drops it while searching', async () => {
    expect(await browser.$(testid('create-ghost')).isExisting()).toBe(true);

    await canvas.search('EXPLAIN');
    expect(await browser.$(testid('create-ghost')).isExisting()).toBe(false);

    await canvas.clearSearch();
  });

  it('searches without leaving the folder', async () => {
    await canvas.search('EXPLAIN');
    expect(await canvas.titles()).toEqual(['EXPLAIN lent sur join']);

    // A note of the same space, but outside this folder, stays out of reach.
    await canvas.search('Dump');
    expect(await canvas.titles()).toEqual([]);
    expect(await crumb.isShowing()).toBe(true);

    await canvas.clearSearch();
  });

  /** One of exactly two places that file a new note; the palette is not one of them. */
  it('gives a note made here the folder it was made in', async () => {
    await canvas.createSnippet();
    await editor.setTitle('Cache hit ratio');
    await editor.close();

    // ⚠️ Two conditions and not one: the note has to exist before "filed" means anything,
    // and keeping them apart is what tells a note never written from one written loose.
    await eventually(
      () => bridge.queryNotes(query({ spaceId, search: 'Cache hit ratio' })),
      (found) => found.matched > 0,
      'the note made here to be written at all',
    );

    const view = await eventually(
      () => bridge.queryNotes(query({ spaceId, folderId: perfId })),
      (filed) =>
        filed.sections.flatMap((section) => section.notes).some((note) => note.title === 'Cache hit ratio'),
      'the note made here to arrive filed',
    );
    expect(view.sections.flatMap((section) => section.notes).map((note) => note.title)).toContain(
      'Cache hit ratio',
    );
  });

  it('renames the folder from beside the breadcrumb', async () => {
    await crumb.rename('Performance');

    expect(
      await eventually(
        () => crumb.name(),
        (name) => name === 'Performance',
        'the breadcrumb to carry the new name',
      ),
    ).toBe('Performance');
    expect((await bridge.listFolders(spaceId)).map((folder) => folder.name)).toContain('Performance');
  });

  /**
   * ⚠️ Escape falls through: the search first, then out of the folder. Leaving is the
   * biggest of the two, so it goes last.
   */
  it('gives the search back before it gives the folder back', async () => {
    await canvas.search('EXPLAIN');
    // ⚠️ The canvas keyboard ignores a keystroke aimed at the search field, by design.
    await blurField();

    await press('Escape');
    await waitForCanvas();
    expect(await canvas.searchQuery()).toBe('');
    expect(await crumb.isShowing()).toBe(true);

    await press('Escape');
    await waitForCanvas();
    expect(await crumb.isShowing()).toBe(false);
  });

  it('comes back to the board it was opened from', async () => {
    expect(await board.isShowing()).toBe(true);
  });

  it('closes again through the breadcrumb itself', async () => {
    await board.openZone('Performance');
    await waitForCanvas();
    expect(await crumb.isShowing()).toBe(true);

    await crumb.back();
    await waitForCanvas();

    expect(await crumb.isShowing()).toBe(false);
    expect(await board.isShowing()).toBe(true);
  });

  /** Deleting from here goes back to the board, with the notes now loose on it. */
  it('leaves the notes standing when the folder is deleted from inside it', async () => {
    await board.openZone('Performance');
    await waitForCanvas();
    const before = (await bridge.queryNotes(query({ spaceId }))).matched;

    await crumb.remove();

    await eventually(
      () => crumb.isShowing(),
      (showing) => !showing,
      'the breadcrumb to go with the folder it named',
    );
    expect((await bridge.listFolders(spaceId)).map((folder) => folder.name)).not.toContain('Performance');
    expect((await bridge.queryNotes(query({ spaceId }))).matched).toBe(before);
  });
});
