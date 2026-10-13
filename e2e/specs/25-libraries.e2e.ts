import { $, $$, expect } from '@wdio/globals';

import { board } from '../pageobjects/board.page.js';
import { spaces } from '../pageobjects/sidebar.page.js';
import {
  PASSPHRASE,
  RELOAD_TIMEOUT,
  eventually,
  pickChoice,
  press,
  setField,
  testid,
  waitForCanvas,
} from '../support/app.js';
import { bridge, draft } from '../support/bridge.js';

/**
 * Several libraries, and switching between them.
 *
 * It needs the gate of a library that has no key file yet — what a first launch shows, and
 * what `24-forgotten-passphrase` leaves behind it — and creates that first library, then a
 * second.
 */
describe('Several libraries', () => {
  const OTHER = 'Boulot';
  /** Only the first library holds it, which is what tells the two rails apart. */
  const HOME = 'Maison';
  const HOME_NOTE = 'Chez moi';

  const rows = () => $$(testid('library-row'));

  /** Through the File menu, which only exists while a library is open. */
  async function openPanel(): Promise<void> {
    await $(testid('file-menu')).click();
    await $(testid('file-libraries')).click();
    await $(testid('library-row')).waitForExist({ timeout: 10_000 });
  }

  /** What the gate says it is asking for — the name, or the menu holding it. */
  const gateLibrary = () => $(testid('vault-library')).getText();

  async function passTheGate(): Promise<void> {
    await setField(testid('vault-passphrase'), PASSPHRASE);
    if (await $(testid('vault-confirmation')).isExisting()) {
      await setField(testid('vault-confirmation'), PASSPHRASE);
    }
    await $(testid('vault-submit')).click();
    await $(testid('canvas')).waitForExist({ timeout: 60_000 });
  }

  before(async () => {
    await eventually(
      () => $(testid('vault-confirmation')).isExisting(),
      (showing) => showing,
      'the gate of a library with no key file yet',
    );
    await passTheGate();

    // After the seeding, which only runs on a library holding no space at all.
    await eventually(
      () => bridge.listSpaces(),
      (all) => all.length > 0,
      'the samples of a first launch',
    );
    const home = await bridge.createSpace({ name: HOME });
    await bridge.createNote(draft({ spaceId: home.id, title: HOME_NOTE }));
  });

  it('lists the one that is open, and says so', async () => {
    await openPanel();

    expect(await rows().length).toBe(1);
    expect(await $(testid('library-open')).isExisting()).toBe(true);
  });

  /**
   * Creating opens it: one gesture rather than two, and the gate then asks for a
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
        RELOAD_TIMEOUT,
      ),
    ).toBe(true);
    // A library with no key file yet asks twice, like a first launch.
    expect(await $(testid('vault-confirmation')).isExisting()).toBe(true);
    // And says which one: there are two now, each with a phrase of its own.
    expect(await gateLibrary()).toContain(OTHER);

    await passTheGate();
  });

  /** Nothing crosses: its own database, its own samples, its own everything. */
  it('lands in a library of its own, seeded as a first launch', async () => {
    const titles = await $$(testid('note-card-title')).map((card) => card.getText());

    expect(titles.length).toBeGreaterThan(0);
    await spaces.open();
    expect(await spaces.names()).not.toContain(HOME);
    await openPanel();
    expect(await rows().length).toBe(2);
  });

  /**
   * The switch rebuilds the front end: a request typed in a tab and never saved went with
   * it, without a word. The shell asks first, whatever area is on screen — and the same
   * question stands before a quit, which no scenario can make: it ends the one process every
   * spec file shares, and is held by the Rust tests of `desktop` instead.
   */
  it('asks before a switch takes an unsaved request with it, then switches back through the gate', async () => {
    await $(testid('libraries-close')).click();
    await $(testid('library-row')).waitForExist({ reverse: true, timeout: 10_000 });
    // A request begun in the HTTP area and left there, then back to the notes.
    await press('3', ['Control']);
    await $(testid('http-tab-new')).waitForDisplayed({ timeout: 10_000 });
    await $(testid('http-tab-new')).click();
    await $(testid('http-tab-dirty')).waitForExist({ timeout: 10_000 });
    await press('1', ['Control']);
    await $(testid('http-page')).waitForExist({ reverse: true, timeout: 10_000 });
    const open = async () => (await bridge.listLibraries()).open;
    const here = await open();

    await openPanel();
    await $(testid('library-switch')).click();
    await $(`${testid('http-leave-dialog')}[data-reason="switch"]`).waitForDisplayed({ timeout: 10_000 });
    expect((await $$(testid('http-leave-name')).getElements()).length).toBe(1);
    expect(await open()).toBe(here);

    // « Annuler »: the library has not changed, and no gate came up.
    await $(testid('http-leave-cancel')).click();
    await $(testid('http-leave-dialog')).waitForExist({ reverse: true, timeout: 10_000 });
    expect(await open()).toBe(here);
    expect(await $(testid('vault-passphrase')).isExisting()).toBe(false);

    // Asked again, and told to go without saving.
    await $(testid('library-switch')).click();
    await $(testid('http-leave-dialog')).waitForDisplayed({ timeout: 10_000 });
    await $(testid('http-leave-discard')).click();

    expect(
      await eventually(
        () => $(testid('vault-passphrase')).isExisting(),
        (showing) => showing,
        'the gate to ask for the other library passphrase',
        RELOAD_TIMEOUT,
      ),
    ).toBe(true);
  });

  /**
   * The File menu does not exist until a library is open, so the gate is the only place
   * someone holding several can say which one they have the phrase for.
   */
  it('goes to another library from the gate itself, and back', async () => {
    const { libraries } = await bridge.listLibraries();
    const other = libraries.find((entry) => entry.name === OTHER);
    const first = libraries.find((entry) => entry.name !== OTHER);

    expect(await gateLibrary()).not.toContain(OTHER);

    await pickChoice('vault-library', other?.id ?? '');
    expect(
      await eventually(
        gateLibrary,
        (text) => text.includes(OTHER),
        'the gate to ask for the other library',
        RELOAD_TIMEOUT,
      ),
    ).toContain(OTHER);

    await pickChoice('vault-library', first?.id ?? '');
    expect(
      await eventually(
        gateLibrary,
        (text) => !text.includes(OTHER),
        'the gate to come back to the first',
        RELOAD_TIMEOUT,
      ),
    ).not.toContain(OTHER);
  });

  it('finds what was left there', async () => {
    await passTheGate();
    await openPanel();

    // The badge moved: the one that was open is not the one that is open now.
    const open = await $(testid('library-open')).getText();
    expect(open.length).toBeGreaterThan(0);
    await $(testid('libraries-close')).click();
    await $(testid('library-row')).waitForExist({ reverse: true, timeout: 10_000 });
  });

  /**
   * Back to a library that was already seeded: the rail and the board must be that library's,
   * not whatever the stores held from the other one.
   */
  it('draws the spaces and the board of the library it came back to', async () => {
    await spaces.open();
    expect(
      await eventually(
        () => spaces.names(),
        (names) => names.includes(HOME),
        'the rail to list this library own spaces',
      ),
    ).toContain(HOME);

    const home = (await bridge.listSpaces()).find((space) => space.name === HOME);
    await spaces.option(home?.id ?? '').click();
    await waitForCanvas();
    await board.show('board');

    expect(await board.looseTitles()).toEqual([HOME_NOTE]);

    await board.show('list');
  });

  /**
   * Named before it runs, and the confirm is somewhere other than the button that
   * fired it — the treatment emptying the trash gets, because it takes everything at once.
   */
  it('names what a deletion would take before it takes it', async () => {
    await openPanel();
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

  /** The gate would have nothing to offer if the last one could go. */
  it('offers no deletion on the one that is left', async () => {
    expect(await $(testid('library-delete')).isExisting()).toBe(false);

    await $(testid('libraries-close')).click();
    // The panel closing is its own condition, so it is waited on rather than slept past.
    await $(testid('library-row')).waitForExist({ reverse: true, timeout: 10_000 });
  });
});
