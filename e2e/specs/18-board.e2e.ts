import { browser, expect } from '@wdio/globals';

import { canvas } from '../pageobjects/canvas.page.js';
import { board, folders, spaces } from '../pageobjects/overlays.page.js';
import { eventually, press, reloadCanvas, testid, waitForCanvas } from '../support/app.js';
import { bridge, draft, homeSpaceId, query } from '../support/bridge.js';

/**
 * The board is drawn from geometry a real database stores, and the first layout is
 * materialised on the first read — only a real one proves it comes back unchanged.
 *
 * ⚠️ A space of its own, and not the home one: seventeen spec files have run before this
 * and left their notes there, so "the loose cards are exactly these" would be a claim
 * about the whole corpus. The e2e run shares one process and one database.
 */
describe('The board', () => {
  let homeId = '';
  let spaceId = '';
  let perfId = '';

  /** ⚠️ The active space is front-end state, so a refresh drops it back to "all spaces". */
  async function reloadInSpace(id = spaceId): Promise<void> {
    await reloadCanvas();
    await spaces.open();
    await spaces.option(id).click();
    await waitForCanvas();
  }

  before(async () => {
    await canvas.open();
    homeId = await homeSpaceId();
    spaceId = (await bridge.createSpace({ name: 'Tableau' })).id;

    perfId = (await bridge.createFolder({ spaceId, name: 'Perf' })).id;
    await bridge.createFolder({ spaceId, name: 'Migrations' });

    const filed = await bridge.createNote(draft({ spaceId, title: 'EXPLAIN lent sur join' }));
    await bridge.fileNotes([filed.id], perfId);
    await bridge.createNote(draft({ spaceId, title: 'Dump nocturne' }));

    await reloadInSpace();
  });

  after(async () => {
    await board.show('date');
    // Takes its folders with it through the cascade, and its notes to the refuge.
    await bridge.deleteSpace(spaceId, homeId);
    await reloadCanvas();
    await spaces.open();
    await spaces.allOption().click();
    await reloadCanvas();
  });

  it('starts on the date view, which stays the default', async () => {
    expect(await board.isShowing()).toBe(false);
    expect(await board.pressed('date')).toBe('true');
  });

  /** ⚠️ A folder belongs to a space, so there would be no zones to draw. */
  it('offers no board while the user is on all spaces', async () => {
    await spaces.open();
    await spaces.allOption().click();
    await waitForCanvas();

    expect(await board.option('board').isEnabled()).toBe(false);

    await spaces.open();
    await spaces.option(spaceId).click();
    await waitForCanvas();
  });

  it('draws every folder as a zone holding its notes, the loose ones beside them', async () => {
    await board.show('board');

    expect(await board.zoneNames()).toEqual(['Perf', 'Migrations']);
    expect(await board.zoneTitles('Perf')).toEqual(['EXPLAIN lent sur join']);
    expect(await board.zoneTitles('Migrations')).toEqual([]);
    expect(await board.looseTitles()).toEqual(['Dump nocturne']);
  });

  /** A card is the same card in both views — full size, tag, snippet, footer and all. */
  it('draws the same card the canvas draws', async () => {
    const card = await board.zoneCard('Perf', 'EXPLAIN lent sur join');

    expect(await card.$(testid('note-card-title')).isExisting()).toBe(true);
    expect(await card.$(testid('note-card-open')).isExisting()).toBe(true);
  });

  /** ⚠️ Dimmed in place: reflowing throws away the only thing the board has. */
  it('dims what a search does not match rather than removing it', async () => {
    await canvas.search('EXPLAIN');

    await eventually(
      () => board.isDimmed('Dump nocturne'),
      (dimmed) => dimmed,
      'the board to dim what the search does not match',
    );
    expect(await board.zoneTitles('Perf')).toEqual(['EXPLAIN lent sur join']);
    expect(await board.looseTitles()).toEqual(['Dump nocturne']);
    expect(await board.isDimmed('Dump nocturne')).toBe(true);
    expect(await board.isDimmed('EXPLAIN lent sur join')).toBe(false);

    await canvas.clearSearch();
    await eventually(
      () => board.isDimmed('Dump nocturne'),
      (dimmed) => !dimmed,
      'the card stayed dimmed after the search was cleared',
    );
  });

  /** The board would look shuffled at every launch otherwise. */
  it('keeps its layout across a restart of the front end', async () => {
    const before = await board.zoneFrames();
    expect(before).toHaveLength(2);

    await reloadInSpace();
    await board.waitForBoard();

    expect(await board.zoneFrames()).toEqual(before);
  });

  it('lays a zone out beside the others rather than on top of them', async () => {
    const frames = await board.zoneFrames();

    expect(frames[0]!.left).not.toBe(frames[1]!.left);
  });

  /** It comes back on the board because the preference is written down, per space. */
  it('comes back on the board after a restart of the front end', async () => {
    await reloadInSpace();

    await board.waitForBoard();
    expect(await board.isShowing()).toBe(true);
  });

  /** Per space, so arranging one does not switch the others. */
  it('remembers the chosen view space by space', async () => {
    await spaces.open();
    await spaces.option(homeId).click();
    await waitForCanvas();
    expect(await board.isShowing()).toBe(false);

    await spaces.open();
    await spaces.option(spaceId).click();
    await board.waitForBoard();
    expect(await board.isShowing()).toBe(true);
  });

  it('leaves the date view exactly as it was when it switches back', async () => {
    await board.show('date');

    await canvas.waitForCard('Dump nocturne');
    expect((await canvas.titles()).sort()).toEqual(['Dump nocturne', 'EXPLAIN lent sur join']);
    expect((await canvas.sectionKeys()).length).toBeGreaterThan(0);

    await board.show('board');
  });

  /**
   * ⚠️ A card on the board is the same card, so its checkboxes are real ones — and the
   * board has to hear about the write, which used to reload the canvas and nothing else.
   */
  it('ticks a todo list on the board, and shows it ticked without a view switch', async () => {
    const list = await bridge.createNote(
      draft({
        spaceId,
        title: 'Avant la release',
        kind: 'checklist',
        items: [
          { text: 'Tag', done: false },
          { text: 'Notes', done: false },
        ],
      }),
    );
    await reloadInSpace();
    await board.show('board');
    await board.waitForBoard();

    const card = browser.$(`${testid('note-card')}[data-note-id="${list.id}"]`);
    await card.$(testid('note-card-item')).click();

    const view = await eventually(
      () => bridge.queryNotes(query({ spaceId, search: 'Avant la release' })),
      (read) => read.sections[0]?.notes[0]?.items?.[0]?.done === true,
      'the tick to reach the note behind the board',
    );
    expect(view.sections[0]?.notes[0]?.items?.[0]?.done).toBe(true);
    // Drawn from what the board re-read, not from the date view behind it.
    expect(await card.$(testid('note-card-item')).getAttribute('aria-checked')).toBe('true');
  });

  /**
   * ⚠️ The grip used to be drawn on top of the selection tick, with an opaque background
   * and a higher `z-index`, so ticking a card on the board meant aiming at the few pixels
   * of checkbox that stuck out from under it. There is no grip at all now — the card is
   * its own handle — and the corner is the tick's.
   */
  it('leaves the corner to the selection tick, having no grip left', async () => {
    await board.show('board');
    expect(await browser.$(testid('board-card-grip')).isExisting()).toBe(false);

    await canvas.check('Dump nocturne');
    expect(await canvas.isChecked('Dump nocturne')).toBe(true);

    await canvas.check('Dump nocturne');
    expect(await canvas.isChecked('Dump nocturne')).toBe(false);
  });

  /**
   * ⚠️ The board lays its cards out zone by zone, where the date view orders them pinned
   * first and then by `updated_at`. The arrows measured the grid in DOM order and resolved
   * the answer through `visibleNotes` — the date view's list — so the first ArrowRight here
   * jumped two cards sideways, onto one nowhere near the pointer.
   *
   * Asserted as a relationship rather than from a fixed start: these files share a session,
   * so what is focused when this runs is whatever the scenario before it left.
   */
  it('walks the cards as they are on screen, not as the date view lists them', async () => {
    await board.show('board');
    const order = await canvas.titles();
    expect(order.length).toBeGreaterThan(1);

    // ⚠️ Walked to a known end rather than started from wherever: these files share one
    // session, so what holds the focus here is whatever the scenario before it left.
    for (const _ of order) {
      await press('ArrowLeft');
    }

    // The first card **on screen**. The date view lists the newest note first, and on this
    // board that is the last of the three — which is what the focus used to land on.
    expect(await canvas.focusedCardTitle()).toBe(order[0]);

    await press('ArrowRight');

    expect(await canvas.focusedCardTitle()).toBe(order[1]);
  });

  /** Nothing is removed from the header: the board is a second view, not a replacement. */
  it('keeps the folder switcher and the rails working beside it', async () => {
    await folders.open();
    expect(await folders.names()).toContain('Perf');
    await folders.close();
  });

  /**
   * ⚠️ Hit-tested, not dispatched: `board.drawZone` fires its events on the surface itself,
   * so it draws wherever it is told to. What the user met was the pointer landing on the
   * dotted ground **beside** a surface the back end had sized to its content (#320).
   */
  describe('on a window larger than what it holds', () => {
    let restore = { width: 1100, height: 720 };

    before(async () => {
      restore = await browser.getWindowSize();
      await browser.setWindowSize(1600, 1000);
      await reloadInSpace();
      await board.show('board');
    });

    after(async () => {
      await browser.setWindowSize(restore.width, restore.height);
      await reloadInSpace();
    });

    it('takes the pointer on the whole of the dotted ground', async () => {
      const missed = await browser.execute(
        (groundSelector: string, surfaceSelector: string) => {
          const ground = document.querySelector(groundSelector)?.getBoundingClientRect();
          const surface = document.querySelector(surfaceSelector);
          if (!ground || !surface) throw new Error('no board');

          // Far right, far down and the far corner — clear of the scrollbars and of the
          // tidy control floating in the bottom-right.
          const points = [
            [ground.left + ground.width * 0.92, ground.top + ground.height * 0.4],
            [ground.left + ground.width * 0.4, ground.top + ground.height * 0.92],
            [ground.left + ground.width * 0.7, ground.top + ground.height * 0.8],
          ];
          return points
            .filter(([x, y]) => !surface.contains(document.elementFromPoint(x, y)))
            .map(([x, y]) => `${Math.round(x)},${Math.round(y)}`);
        },
        testid('board'),
        testid('board-surface'),
      );

      expect(missed).toEqual([]);
    });
  });
});
