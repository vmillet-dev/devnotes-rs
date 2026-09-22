import { $, browser, expect } from '@wdio/globals';

import { eventually, testid } from '../support/app.js';

/**
 * The way out of a library nobody can open any more.
 *
 * ⚠️ This file runs **after `23-backups`** and depends on it: that one ends by restoring
 * a copy, which closes the library and leaves the gate on screen. There is no other way
 * to meet a locked gate inside one run — the unlocked state lives in the process, and the
 * process outlives every page reload. Nothing may be filed after this one either: it
 * archives the library and leaves the profile with none.
 */
describe('A forgotten passphrase', () => {
  const panel = () => $(testid('vault-archive'));

  before(async () => {
    // The precondition, said out loud rather than assumed: a green run of 22 is what
    // puts it there, and a failure here should read as "the gate never came" and not as
    // a mystery about a missing button.
    await eventually(
      () => $(testid('vault-passphrase')).isExisting(),
      (showing) => showing,
      'the locked gate left by the restore in 22-backups',
    );
  });

  /** ⚠️ Well away from the field: it is the one thing that cannot help here. */
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
   * ⚠️ The key file travels with the notes, so what is left has no library at all — which
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
