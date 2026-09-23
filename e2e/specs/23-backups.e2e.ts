import { $, $$, browser, expect } from '@wdio/globals';

import { canvas } from '../pageobjects/canvas.page.js';
import { fileMenu, settings } from '../pageobjects/titlebar.page.js';
import { eventually, testid } from '../support/app.js';

/**
 * The copies, and the one gesture that puts one back.
 *
 * ⚠️ **Last on purpose.** The final scenario really replaces the library and leaves it
 * locked, which every other file would then meet as a gate it was not written for. The
 * numeric prefix is the run order, and nothing may be filed after this one.
 */
describe('The backup copies', () => {
  before(canvas.open);

  const rows = () => $$(testid('backup-row'));
  const restore = () => $(testid('backup-restore'));
  const confirmStrip = () => $(testid('backup-confirm'));

  beforeEach(async () => {
    await fileMenu.openPreferences();
    await settings.open('security');
  });

  /**
   * ⚠️ The whole of #251: the copy was taken at unlock, kept beside the library and
   * pruned to three, and none of it was visible anywhere.
   */
  it('lists the copy the launch took, with when and how big', async () => {
    const listed = await eventually(
      () => rows().length,
      (count) => count > 0,
      'the launch copy to be listed',
    );
    expect(listed).toBeGreaterThan(0);

    const text = await rows()[0]!.getText();
    // The stamp is the folder's own name, so it is the one label no translation touches.
    expect(text).toMatch(/\d{4}-\d{2}-\d{2}_\d{2}-\d{2}-\d{2}/);
    expect(text).toMatch(/\d+(\.\d+)? [KM]o/);

    await settings.close();
  });

  /**
   * ⚠️ The trigger only proposes. A second click on the button that fired it is the guard
   * a double click defeats, and this is the one gesture that replaces a whole corpus.
   */
  it('names what a restore would do rather than running one', async () => {
    await restore().click();

    expect(await confirmStrip().isExisting()).toBe(true);
    expect(await confirmStrip().getText()).toMatch(/\d{4}-\d{2}-\d{2}_\d{2}-\d{2}-\d{2}/);
    // The canvas is still there, which is the proof nothing was replaced.
    expect(await $(testid('canvas')).isExisting()).toBe(true);

    await $(testid('backup-cancel')).click();
    expect(await confirmStrip().isExisting()).toBe(false);
    expect(await restore().isExisting()).toBe(true);

    await settings.close();
  });

  /**
   * ⚠️ The library is closed by the time the command answers, so the shell goes back to
   * the gate: the restored copy needs a passphrase, and asking for it is the only proof
   * the right file is in place. Nothing is unlocked afterwards — this file is last, and
   * the profile is wiped before the next run.
   */
  it('replaces the library and sends the window back to the gate', async () => {
    await restore().click();
    await $(testid('backup-confirm-restore')).click();

    expect(
      await eventually(
        () => $(testid('vault-passphrase')).isExisting(),
        (showing) => showing,
        'the gate to come back once the copy is in place',
      ),
    ).toBe(true);

    // The File menu is gated too, so the panel the restore was asked from is gone with it.
    expect(await $(testid('file-menu')).isExisting()).toBe(false);
    expect(await browser.$(testid('canvas')).isExisting()).toBe(false);
  });
});
