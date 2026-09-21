import { browser, expect } from '@wdio/globals';

import { canvas } from '../pageobjects/canvas.page.js';
import { editor } from '../pageobjects/editor.page.js';
import { trash, undoBar } from '../pageobjects/overlays.page.js';
import { eventually, press, reloadCanvas, testid } from '../support/app.js';
import { bridge, draft, homeSpaceId, query } from '../support/bridge.js';

/**
 * Deleting a note does not delete it: `deleted_at` is stamped and retention decides
 * later. Every read has to filter on it.
 */
describe('Deleting a note, and taking it back', () => {
  let spaceId = '';

  before(async () => {
    await canvas.open();
    spaceId = await homeSpaceId();
  });

  async function seed(title: string): Promise<void> {
    await bridge.createNote(draft({ spaceId, title }));
    await reloadCanvas();
    await canvas.waitForCard(title);
  }

  it('needs the second click to delete anything', async () => {
    await seed('Armed but not fired');
    const card = await canvas.openCardMenu('Armed but not fired');
    await card.$('[data-testid="note-card-delete"]').click();

    // ⚠️ A duration, deliberately: this asserts the note is *still* there, and nothing
    // happening is not a condition anything can wait on.
    await browser.pause(500);
    expect((await bridge.queryNotes(query({ search: 'Armed but not fired' }))).matched).toBe(1);

    // Leave nothing armed for the next test.
    await press('Escape');
  });

  /**
   * ⚠️ The keyboard asks twice too. One press used to trash whichever card the ring was
   * on — and after opening a note and scrolling, that card can be nowhere on screen.
   */
  it('needs the second Delete too, and says so on the card', async () => {
    await seed('Armed from the keyboard');
    await canvas.openNote('Armed from the keyboard');
    await editor.close();

    await press('Delete');

    const card = await canvas.cardWithTitle('Armed from the keyboard');
    expect(await card.$(testid('note-card-arming')).isExisting()).toBe(true);
    expect((await bridge.queryNotes(query({ search: 'Armed from the keyboard' }))).matched).toBe(1);

    await press('Delete');

    expect(
      await eventually(
        () => bridge.queryNotes(query({ search: 'Armed from the keyboard' })),
        (view) => view.matched === 0,
        'the second press to reach the trash',
      ),
    ).toBeTruthy();
    await undoBar.dismiss();
  });

  /**
   * ⚠️ Escape has always disarmed — it is the first rung of the fall-through — and the
   * banner named only the key that goes through, so the only way out a user could see was
   * the cancel entry in the card's own menu, which is a mouse target (#281).
   */
  it('lets Escape call the armed deletion off', async () => {
    await seed('Armed then called off');
    await canvas.openNote('Armed then called off');
    await editor.close();

    await press('Delete');
    const card = await canvas.cardWithTitle('Armed then called off');
    expect(await card.$(testid('note-card-arming')).isExisting()).toBe(true);

    await press('Escape');
    await card.$(testid('note-card-arming')).waitForExist({ reverse: true });

    // Back to zero, not to one: the next Delete arms again rather than trashing.
    await press('Delete');
    expect(await card.$(testid('note-card-arming')).isExisting()).toBe(true);
    expect((await bridge.queryNotes(query({ search: 'Armed then called off' }))).matched).toBe(1);

    await press('Escape');
  });

  /**
   * ⚠️ The report: the card says "Suppr. à nouveau · Échap pour annuler", the second
   * Suppr sends the note to the trash, and the key the card had just taught meant nothing
   * one keystroke later — the reversal was a button and a shortcut nothing named (#293).
   */
  it('takes the note back on Escape while the bar is still offering', async () => {
    await seed('Taken back with Escape');
    await canvas.openNote('Taken back with Escape');
    await editor.close();

    await press('Delete');
    await press('Delete');
    await undoBar.bar().waitForExist({ timeout: 10_000 });

    await press('Escape');

    await canvas.waitForCard('Taken back with Escape');
    expect((await bridge.listTrash()).map((row) => row.title)).not.toContain('Taken back with Escape');
  });

  it('moves the note to the trash rather than dropping it', async () => {
    await seed('Delete me once');
    await canvas.deleteNote('Delete me once');
    await canvas.waitForNoCard('Delete me once');

    expect((await bridge.queryNotes(query({ search: 'Delete me once' }))).matched).toBe(0);
    const trashed = await bridge.listTrash();
    expect(trashed.map((row) => row.title)).toContain('Delete me once');
  });

  it('offers an undo, and honours it', async () => {
    await undoBar.bar().waitForExist({ timeout: 10_000 });
    await undoBar.restore();

    await canvas.waitForCard('Delete me once');
    expect((await bridge.listTrash()).map((row) => row.title)).not.toContain('Delete me once');
  });

  it('keeps the undo available after the banner has gone', async () => {
    await seed('Undo by keyboard');
    await canvas.deleteNote('Undo by keyboard');
    await canvas.waitForNoCard('Undo by keyboard');

    // The banner is what the 8 s timer clears; the record it suggests is not.
    await undoBar.bar().waitForExist({ reverse: true, timeout: 15_000 });
    await press('z', ['Control']);

    await canvas.waitForCard('Undo by keyboard');
  });

  it('restores from the trash panel', async () => {
    await seed('Restore from panel');
    await canvas.deleteNote('Restore from panel');
    await canvas.waitForNoCard('Restore from panel');
    await undoBar.dismiss();

    await trash.open();
    expect(await trash.titles()).toContain('Restore from panel');
    await trash.restore('Restore from panel');
    await trash.close();

    await canvas.waitForCard('Restore from panel');
  });

  it('purges one row for good, which the retention no longer protects', async () => {
    await seed('Purge me');
    await canvas.deleteNote('Purge me');
    await canvas.waitForNoCard('Purge me');
    await undoBar.dismiss();

    await trash.open();
    await trash.purge('Purge me');
    await eventually(
      () => trash.titles(),
      (titles) => !titles.includes('Purge me'),
      'the purged row never left the panel',
    );
    await trash.close();

    expect((await bridge.listTrash()).map((row) => row.title)).not.toContain('Purge me');
  });

  it('empties the whole trash, and says so once it is empty', async () => {
    await seed('Swept away');
    await seed('Swept away too');
    await canvas.deleteNote('Swept away');
    await canvas.waitForNoCard('Swept away');
    await undoBar.dismiss();
    await canvas.deleteNote('Swept away too');
    await canvas.waitForNoCard('Swept away too');
    await undoBar.dismiss();

    await trash.open();
    expect((await trash.titles()).length).toBeGreaterThan(1);

    await trash.empty();

    // The empty state replaces the list rather than leaving a header over nothing.
    await eventually(
      () => trash.emptyState().isExisting(),
      (showing) => showing,
      'the trash to say it is empty',
    );
    expect(await trash.rows().length).toBe(0);
    expect(await bridge.listTrash()).toHaveLength(0);
    await trash.close();
  });
});
