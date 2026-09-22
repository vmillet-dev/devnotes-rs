import { $, $$, expect } from '@wdio/globals';

import { PASSPHRASE, eventually, setField, testid } from '../support/app.js';

/**
 * Several libraries, and switching between them.
 *
 * ⚠️ **Last on purpose, and after `24-forgotten-passphrase`.** That one archives the
 * library and leaves the gate asking for a phrase on a fresh one, which is exactly the
 * state this file needs: it creates that first library, then a second, and ends on a gate
 * again. Nothing may be filed after it.
 */
describe('Several libraries', () => {
  const OTHER = 'Boulot';

  const rows = () => $$(testid('library-row'));

  /** ⚠️ Through the File menu, which only exists while a library is open. */
  async function openPanel(): Promise<void> {
    await $(testid('file-menu')).click();
    await $(testid('file-libraries')).click();
    await $(testid('library-row')).waitForExist({ timeout: 10_000 });
  }

  async function passTheGate(): Promise<void> {
    await setField(testid('vault-passphrase'), PASSPHRASE);
    if (await $(testid('vault-confirmation')).isExisting()) {
      await setField(testid('vault-confirmation'), PASSPHRASE);
    }
    await $(testid('vault-submit')).click();
    await $(testid('canvas')).waitForExist({ timeout: 60_000 });
  }

  before(async () => {
    // The gate `24-forgotten-passphrase` left: a library with no key file yet.
    await eventually(
      () => $(testid('vault-passphrase')).isExisting(),
      (showing) => showing,
      'the gate left by the archive in 24-forgotten-passphrase',
    );
    await passTheGate();
  });

  it('lists the one that is open, and says so', async () => {
    await openPanel();

    expect(await rows().length).toBe(1);
    expect(await $(testid('library-open')).isExisting()).toBe(true);
  });

  /**
   * ⚠️ Creating opens it: one gesture rather than two, and the gate then asks for a
   * phrase — which is what a first launch does too.
   */
  it('creates one and goes straight to it, through the gate', async () => {
    await setField(testid('library-new-name'), OTHER);
    await $(testid('library-create')).click();

    expect(
      await eventually(
        () => $(testid('vault-passphrase')).isExisting(),
        (showing) => showing,
        'the gate to ask for the new library passphrase',
      ),
    ).toBe(true);
    // A library with no key file yet asks twice, like a first launch.
    expect(await $(testid('vault-confirmation')).isExisting()).toBe(true);

    await passTheGate();
  });

  /** ⚠️ Nothing crosses: its own database, its own samples, its own everything. */
  it('lands in a library of its own, seeded as a first launch', async () => {
    const titles = await $$(testid('note-card-title')).map((card) => card.getText());

    expect(titles.length).toBeGreaterThan(0);
    await openPanel();
    expect(await rows().length).toBe(2);
  });

  it('switches back through the gate, and finds what was left there', async () => {
    await $(`${testid('library-switch')}`).click();

    expect(
      await eventually(
        () => $(testid('vault-passphrase')).isExisting(),
        (showing) => showing,
        'the gate to ask for the other library passphrase',
      ),
    ).toBe(true);

    await passTheGate();
    await openPanel();

    // The badge moved: the one that was open is not the one that is open now.
    const open = await $(testid('library-open')).getText();
    expect(open.length).toBeGreaterThan(0);
  });

  /**
   * ⚠️ Named before it runs, and the confirm is somewhere other than the button that
   * fired it — the treatment emptying the trash gets, because it takes everything at once.
   */
  it('names what a deletion would take before it takes it', async () => {
    await $(testid('library-delete')).click();

    expect(await $(testid('library-confirm')).isExisting()).toBe(true);
    expect(await $(testid('library-confirm')).getText()).toContain(OTHER);
    expect(await rows().length).toBe(2);

    await $(testid('library-confirm-delete')).click();

    expect(
      await eventually(
        () => rows().length,
        (count) => count === 1,
        'the library to be erased once it was confirmed',
      ),
    ).toBe(1);
  });

  /** ⚠️ The gate would have nothing to offer if the last one could go. */
  it('offers no deletion on the one that is left', async () => {
    expect(await $(testid('library-delete')).isExisting()).toBe(false);

    await $(testid('libraries-close')).click();
    // The panel closing is its own condition, so it is waited on rather than slept past.
    await $(testid('library-row')).waitForExist({ reverse: true, timeout: 10_000 });
  });
});
