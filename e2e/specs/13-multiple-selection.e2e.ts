import { expect } from '@wdio/globals';

import { canvas } from '../pageobjects/canvas.page.js';
import { selectionBar } from '../pageobjects/header.page.js';
import { undoBar } from '../pageobjects/overlays.page.js';
import { banners } from '../pageobjects/titlebar.page.js';
import { clipboardText, eventually, reloadCanvas } from '../support/app.js';
import { bridge, draft, homeSpaceId, query } from '../support/bridge.js';

/**
 * A batch never fails whole for one stale id — a selection can hold an id that went stale
 * between the click and the call. `move_notes` and `tag_notes` answer **what they
 * changed**, which is what the undo hands back.
 */
describe('Selecting several notes at once', () => {
  const first = 'Batch one';
  const second = 'Batch two';
  const untouched = 'Not in the batch';
  let spaceId = '';
  let refugeId = '';

  before(async () => {
    await canvas.open();
    spaceId = await homeSpaceId();
    refugeId = (await bridge.createSpace({ name: 'Archive' })).id;

    for (const title of [first, second, untouched]) {
      await bridge.createNote(draft({ spaceId, title }));
    }
    await reloadCanvas();
    await canvas.waitForCard(first);
  });

  async function reread(title: string) {
    const view = await bridge.queryNotes(query({ search: title }));
    return view.sections[0]?.notes[0];
  }

  it('shows no bar until a card is ticked', async () => {
    expect(await selectionBar.bar().isExisting()).toBe(false);
  });

  it('counts what is ticked', async () => {
    await canvas.check(first);
    await selectionBar.bar().waitForExist({ timeout: 10_000 });
    expect(await canvas.isChecked(first)).toBe(true);

    await canvas.check(second);
    // The count is the one thing saying how many notes the next click acts on.
    expect(await selectionBar.count()).toContain('2');
  });

  it('tags every ticked note in one call, and only those', async () => {
    await selectionBar.tag('batch');

    await eventually(
      () => reread(second),
      (note) => note?.tags.includes('batch') === true,
      'the batch to reach the last note it was given',
    );
    expect((await reread(first))?.tags).toEqual(['batch']);
    expect((await reread(second))?.tags).toEqual(['batch']);
    expect((await reread(untouched))?.tags).toEqual([]);
  });

  /**
   * ⚠️ The pairs come back from Rust, not from the selection: `first` already carries
   * `batch`, so undoing a second tagging must not strip it.
   */
  it('offers to take a bulk tagging back, without stripping what was already there', async () => {
    await selectionBar.tag('reversible');
    await eventually(
      () => reread(first),
      (note) => note?.tags.includes('reversible') === true,
      'the tag never reached the first note',
    );

    await undoBar.bar().waitForDisplayed({ timeout: 10_000 });
    await undoBar.restore();
    const undone = await eventually(
      () => reread(first),
      (note) => note?.tags.includes('reversible') === false,
      'the tagging was never undone',
    );

    expect(undone?.tags).toEqual(['batch']);
    expect((await reread(second))?.tags).toEqual(['batch']);
  });

  it('copies the selection as Markdown', async function () {
    await selectionBar.copy();

    // `copyConfirmation` is on by default, and the toast is the only thing saying the
    // copy happened.
    await eventually(
      () => banners.status().isExisting(),
      (showing) => showing,
      'the copy confirmation to appear',
    );

    const copied = await clipboardText();
    if (copied === null) {
      // No readable clipboard on this runner; see `clipboardText`.
      this.skip();
      return;
    }
    expect(copied).toContain(first);
    expect(copied).toContain(second);
    expect(copied).not.toContain(untouched);
  });

  it('moves them to another space through the renamed argument', async () => {
    await selectionBar.moveTo(refugeId);

    await eventually(
      () => reread(second),
      (note) => note?.spaceId === refugeId,
      'the batch to reach the last note it was given',
    );
    expect((await reread(first))?.spaceId).toBe(refugeId);
    expect((await reread(second))?.spaceId).toBe(refugeId);
    expect((await reread(untouched))?.spaceId).toBe(spaceId);
  });

  /**
   * Putting thirty notes back by hand means remembering which thirty, and from where.
   * ⚠️ States its own precondition: the move above took its notes out of this space, and
   * with them the selection.
   */
  it('offers to take a bulk move back, to the space each note left', async () => {
    await reloadCanvas();
    await canvas.waitForCard(untouched);
    await canvas.check(untouched);
    await selectionBar.bar().waitForExist({ timeout: 10_000 });

    await selectionBar.moveTo(refugeId);
    await eventually(
      () => reread(untouched),
      (note) => note?.spaceId === refugeId,
      'the note never reached the refuge',
    );

    await undoBar.bar().waitForDisplayed({ timeout: 10_000 });
    await undoBar.restore();
    const back = await eventually(
      () => reread(untouched),
      (note) => note?.spaceId === spaceId,
      'the move was never undone',
    );

    expect(back?.spaceId).toBe(spaceId);
  });

  it('drops the selection without touching the notes', async () => {
    await reloadCanvas();
    await canvas.check(untouched);
    await selectionBar.bar().waitForExist({ timeout: 10_000 });

    await selectionBar.clear();
    await selectionBar.bar().waitForExist({ reverse: true, timeout: 10_000 });
    expect(await canvas.isChecked(untouched)).toBe(false);
    expect((await reread(untouched))?.tags).toEqual([]);
  });

  it('deletes the batch on the second click, into the trash like a single one', async () => {
    await canvas.check(first);
    await canvas.check(second);
    await selectionBar.bar().waitForExist({ timeout: 10_000 });

    await selectionBar.delete();
    await canvas.waitForNoCard(first);
    await canvas.waitForNoCard(second);

    // `delete_notes` stamps `deleted_at` too — a batch is not a shortcut past the reprieve.
    const trashed = (await bridge.listTrash()).map((row) => row.title);
    expect(trashed).toContain(first);
    expect(trashed).toContain(second);
    expect(trashed).not.toContain(untouched);
  });

  it('survives an id that went stale, answering a count rather than failing', async () => {
    const stale = (await bridge.createNote(draft({ spaceId, title: 'Gone before the call' }))).id;
    const alive = (await bridge.createNote(draft({ spaceId, title: 'Still there' }))).id;
    await bridge.deleteNote(stale);
    await bridge.purgeNotes([stale]);

    // One of the two ids no longer exists. The batch reports what it could do.
    expect(await bridge.deleteNotes([stale, alive])).toBe(1);
  });
});
