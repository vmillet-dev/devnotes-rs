import { $, $$, browser, expect } from '@wdio/globals';

import { canvas } from '../pageobjects/canvas.page.js';
import { fileMenu, settings } from '../pageobjects/titlebar.page.js';
import { eventually, reloadCanvas, testid } from '../support/app.js';
import { bridge, draft, homeSpaceId, query } from '../support/bridge.js';

/**
 * The copies, and the one gesture that puts one back.
 *
 * The final scenario really replaces the library, and unlocks the copy it put back.
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

  /** The copy the launch took is listed where the application can be read: when, and how big. */
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
   * The trigger only proposes. A second click on the button that fired it is the guard
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
   * The library is closed by the time the command answers, so the page reloads to the
   * gate: the restored copy needs a passphrase, and asking for it is the only proof the
   * right file is in place. SEC-09: unlocked, the canvas still listed a note written after
   * the copy, and Preferences opened again.
   */
  it('replaces the library and sends the window back to the gate', async () => {
    const later = 'Written after the copy';
    await settings.close();
    await bridge.createNote(draft({ spaceId: await homeSpaceId(), title: later, content: 'echo later' }));
    await reloadCanvas();
    await canvas.waitForCard(later);
    await fileMenu.openPreferences();
    await settings.open('security');

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
    // Said across the reload: where the library it replaced was set aside.
    await eventually(
      () => $(testid('status-toast')).getAttribute('data-key'),
      (key) => key === 'backups.restored',
      'the restore said at the gate',
    );

    await canvas.open();
    expect(await canvas.titles()).not.toContain(later);
    expect((await bridge.queryNotes(query({ search: later }))).matched).toBe(0);
    expect(await $(testid('settings-close')).isExisting()).toBe(false);
  });
});
