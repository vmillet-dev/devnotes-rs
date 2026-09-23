import { browser, expect } from '@wdio/globals';

import { RAIL_WIDTH } from '@core/services/settings/app-settings.model';
import { canvas } from '../pageobjects/canvas.page.js';
import { rail, spaces } from '../pageobjects/sidebar.page.js';
import { eventually, press, reloadCanvas, testid } from '../support/app.js';
import { bridge, draft, homeSpaceId, query } from '../support/bridge.js';

/**
 * `notes.space_id` carries `ON DELETE CASCADE`, so deleting a space without a refuge
 * would take its notes with it. Only a real database proves the cascade never fires.
 */
describe('Spaces', () => {
  let homeId = '';

  before(async () => {
    await canvas.open();
    homeId = await homeSpaceId();

    // ⚠️ The first scenario asserts on "the only space there is", a property of the whole
    // database — and one application serves the whole run. Established, never inherited.
    for (const space of await bridge.listSpaces()) {
      if (space.id !== homeId) {
        await bridge.deleteSpace(space.id, homeId);
      }
    }
    await reloadCanvas();
  });

  /** The active space outlives the page, and every later file asserts on titles. */
  after(async () => {
    await spaces.open();
    await spaces.allOption().click();
    await reloadCanvas();
  });

  it('refuses to delete the only space there is', async () => {
    await spaces.open();
    await browser.$('[data-testid="space-edit"]').click();

    expect(await spaces.deleteBlocked().isExisting()).toBe(true);
    await press('Escape');
    await spaces.close();
  });

  it('creates a space from the switcher', async () => {
    await spaces.open();
    await spaces.create('Veille');

    const all = await eventually(
      () => bridge.listSpaces(),
      (listed) => listed.some((space) => space.name === 'Veille'),
      'the new space to be listed',
    );
    expect(all.map((space) => space.name)).toContain('Veille');
  });

  it('lists both in the switcher, by name', async () => {
    await spaces.open();
    expect(await spaces.names()).toContain('Veille');
    await spaces.close();
  });

  it('renames one without touching its id', async () => {
    const before = (await bridge.listSpaces()).find((space) => space.name === 'Veille');
    await spaces.open();
    await spaces.rename(before!.id, 'Lectures');

    const after = await eventually(
      async () => (await bridge.listSpaces()).find((space) => space.id === before!.id),
      (space) => space?.name === 'Lectures',
      'the renamed space to come back under its new name',
    );
    expect(after?.name).toBe('Lectures');
  });

  /**
   * The order is SQL's, which only a real database can prove. ⚠️ Its own space, named to
   * sort last: Mocha runs a nested suite after its siblings, and by then the tests above
   * have renamed and deleted theirs.
   */
  describe('pinning one to the head of the list', () => {
    const name = 'Zzz pinned';
    let id = '';

    before(async () => {
      id = (await bridge.createSpace({ name })).id;
      // Written straight through the bridge, so the front end has never heard of it.
      await reloadCanvas();
    });

    after(async () => {
      const refuge = (await bridge.listSpaces()).find((space) => space.id !== id);
      await bridge.deleteSpace(id, refuge!.id);
    });

    it('hoists it whatever its name, and lets it fall back', async () => {
      expect((await bridge.listSpaces()).at(-1)?.id).toBe(id);

      await spaces.open();
      await spaces.togglePin(id);

      const pinned = await eventually(
        () => bridge.listSpaces(),
        (listed) => listed[0]?.pinned === true,
        'the pinned space to reach the head of the list',
      );
      expect(pinned[0]?.id).toBe(id);
      expect(pinned[0]?.pinned).toBe(true);

      await spaces.open();
      await spaces.togglePin(id);

      const loose = await eventually(
        () => bridge.listSpaces(),
        (listed) => listed.at(-1)?.pinned === false,
        'the unpinned space to fall back to its name order',
      );
      expect(loose.at(-1)?.id).toBe(id);
      expect(loose.at(-1)?.pinned).toBe(false);
    });

    /** ⚠️ A rename answers with the row it read back, not with what it was sent. */
    it('survives a rename', async () => {
      await spaces.open();
      await spaces.togglePin(id);
      await eventually(
        () => bridge.listSpaces(),
        (listed) => listed.find((space) => space.id === id)?.pinned === true,
        'the space to be pinned before it is renamed',
      );

      await spaces.open();
      await spaces.rename(id, 'Zzz renamed');

      const after = await eventually(
        async () => (await bridge.listSpaces()).find((space) => space.id === id),
        (space) => space?.name === 'Zzz renamed',
        'the pinned space to come back renamed',
      );
      expect(after?.name).toBe('Zzz renamed');
      expect(after?.pinned).toBe(true);
    });
  });

  it('filters the canvas down to the active space', async () => {
    const target = (await bridge.listSpaces()).find((space) => space.name === 'Lectures')!;
    await bridge.createNote(draft({ spaceId: target.id, title: 'Only in Lectures' }));
    await reloadCanvas();

    await spaces.open();
    await spaces.option(target.id).click();
    await canvas.waitForCard('Only in Lectures');
    expect(await canvas.titles()).toEqual(['Only in Lectures']);

    // The only thing on screen saying the canvas is not showing everything.
    expect(await spaces.label()).toContain('Lectures');
  });

  it('moves the notes out before dropping the space', async () => {
    const target = (await bridge.listSpaces()).find((space) => space.name === 'Lectures')!;
    await spaces.open();
    await spaces.remove(target.id, homeId);

    await eventually(
      () => bridge.listSpaces(),
      (listed) => !listed.some((space) => space.name === 'Lectures'),
      'the absorbed space to be gone',
    );

    // The note survived, in the refuge — a cascade would have taken it.
    const view = await bridge.queryNotes(query({ search: 'Only in Lectures' }));
    expect(view.matched).toBe(1);
    expect(view.sections[0]?.notes[0]?.spaceId).toBe(homeId);
  });

  /**
   * ⚠️ Runs after everything above it, like the pinning suite: Mocha takes a nested suite
   * once its siblings are done, and this one puts the rail away for a moment.
   */
  describe('the library rail', () => {
    after(async () => {
      await rail.show();
    });

    it('is the navigation, so the switchers stay out of the topbar', async () => {
      expect(await rail.isShowing()).toBe(true);
      expect(await browser.$(testid('space-switcher')).isExisting()).toBe(false);
      expect(await browser.$(testid('folder-switcher')).isExisting()).toBe(false);
    });

    /** ⚠️ The drag is pointer events, which no keyboard has: the edge answers arrows too. */
    it('is resized from its edge, and the width survives the page', async () => {
      await rail.show();
      const before = await rail.width();

      await rail.widen();
      const wider = await eventually(
        () => rail.width(),
        (width) => width > before,
        'the rail never widened',
      );

      expect(wider).toBeGreaterThan(before);

      const widened = await rail.width();
      await reloadCanvas();
      expect(await rail.width()).toBe(widened);
    });

    /**
     * ⚠️ The two panels a ⋯ opens used to hold the floor up with a `min-width` of their
     * own, and releasing it is the whole of #212: at the floor the delete control has to
     * still be inside the rail, which only a laid-out window can say.
     */
    it('narrows to its floor with the space panel still inside it', async () => {
      await rail.show();
      await rail.narrow();
      const narrowed = await eventually(
        () => rail.width(),
        (width) => width === RAIL_WIDTH.min,
        'the rail never reached its floor',
      );

      await browser.$(testid('space-edit')).click();

      expect(narrowed).toBe(RAIL_WIDTH.min);
      // The rename submit and not the delete: the delete only exists while another space
      // stands to take the notes in, and it is the same row either way.
      expect(await rail.overflowOf(testid('space-rename-submit'))).toBe(0);
    });

    /** Hidden or shown is a preference, so it has to survive the page it was set on. */
    it('gives the switchers back when it is put away, and is remembered', async () => {
      await rail.hide();
      expect(await browser.$(testid('space-switcher')).isExisting()).toBe(true);

      // ⚠️ The preference has to reach the file before the reload, or the rail comes back.
      await eventually(
        () => rail.isShowing(),
        (showing) => !showing,
        'the rail to be put away',
      );
      await reloadCanvas();

      expect(await rail.isShowing()).toBe(false);
      expect(await browser.$(testid('space-switcher')).isExisting()).toBe(true);
    });
  });
});
