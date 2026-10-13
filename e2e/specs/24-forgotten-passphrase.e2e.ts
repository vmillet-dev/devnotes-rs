import { $, browser, expect } from '@wdio/globals';

import { eventually, testid, waitForCanvas } from '../support/app.js';
import { bridge } from '../support/bridge.js';

/**
 * The way out of a library whose phrase is forgotten.
 *
 * It archives the library and leaves the profile with none, so nothing may run after it in
 * the same application.
 */
describe('A forgotten passphrase', () => {
  const panel = () => $(testid('vault-archive'));

  /**
   * A gate asking for the phrase of a library that has one: the unlocked state outlives
   * every page reload, so the library is closed the way switching to it would, and the
   * front end reloaded onto the gate.
   */
  before(async () => {
    await waitForCanvas();
    const { open } = await bridge.listLibraries();
    if (!open) throw new Error('the registry names no open library');
    await bridge.openLibrary(open);
    await browser.refresh();

    await eventually(
      async () =>
        (await $(testid('vault-passphrase')).isExisting()) &&
        !(await $(testid('vault-confirmation')).isExisting()),
      (locked) => locked,
      'the gate to ask for the phrase of the library it closed',
    );
  });

  /** Well away from the field: it is the one thing that cannot help here. */
  it('offers a way out from under the unlock button', async () => {
    expect(await $(testid('vault-forgotten')).isExisting()).toBe(true);
  });

  it('replaces the field with what would be lost, rather than acting', async () => {
    await $(testid('vault-forgotten')).click();

    expect(await panel().isExisting()).toBe(true);
    expect(await $(testid('vault-passphrase')).isExisting()).toBe(false);
  });

  it('goes back to the field without touching anything', async () => {
    await $(testid('vault-keep-trying')).click();

    expect(await panel().isExisting()).toBe(false);
    expect(await $(testid('vault-passphrase')).isExisting()).toBe(true);
  });

  /**
   * The key file travels with the notes, so what is left has no library at all — which
   * is what turns the gate into the one that asks for a **new** phrase. A gate still
   * asking for the old one would be the same dead end with extra steps.
   */
  it('archives the library and comes back asking for a new phrase', async () => {
    await $(testid('vault-forgotten')).click();
    await $(testid('vault-archive-confirm')).click();

    expect(
      await eventually(
        () => $(testid('vault-confirmation')).isExisting(),
        (asking) => asking,
        'the gate to ask for a phrase of its own',
      ),
    ).toBe(true);

    // Where it went, because "archived" is only true if the user can be told where.
    const report = await browser.$(testid('status-toast')).getText();
    expect(report).toContain('archived');
  });
});
