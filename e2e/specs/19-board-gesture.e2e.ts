import { browser, expect } from '@wdio/globals';

import { canvas } from '../pageobjects/canvas.page.js';
import { board, selectionBar, spaces, undoBar } from '../pageobjects/overlays.page.js';

import { eventually, reloadCanvas, testid, waitForCanvas } from '../support/app.js';
import { bridge, draft, homeSpaceId, query } from '../support/bridge.js';

/**
 * The gesture, against a real database: what a drop files, what a move writes, and what
 * survives a restart of the front end.
 *
 * ⚠️ The pointer events are dispatched from inside the page, like every other input this
 * suite sends — the embedded driver drops WebDriver actions. So these prove the wiring,
 * grip to store to command to database; the WebView's own pointer capture is what the
 * unit specs cover.
 *
 * ⚠️ A space of its own: eighteen files run before this one and leave notes in the home
 * space, so "the loose cards are exactly these" would be a claim about the whole corpus.
 */

/** `folders::board::CARD_HEIGHT`, which `scripts/board-geometry.test.mjs` holds to the CSS. */
const CARD_HEIGHT = 150;

/**
 * `folders::board::ZONE_HEADER`. ⚠️ The one number in that module the sweep cannot read:
 * the header is its padding plus wherever the text lands, and no stylesheet states it. So it
 * is measured here instead, against the assembled application.
 */
const ZONE_HEADER = 38;
describe('Arranging the board', () => {
  let homeId = '';
  let spaceId = '';
  let perfId = '';
  let migrationsId = '';
  let looseId = '';
  let filedId = '';

  /** The frame as the database holds it, which is not what an overlay is drawing. */
  async function storedFrame(folderId: string) {
    const view = await bridge.boardView({
      spaceId,
      search: '',
      filter: 'all',
      tags: [],
      languages: [],
      now: new Date().toISOString(),
    });
    return view.zones.find((zone) => zone.folder.id === folderId)?.frame ?? null;
  }

  async function openBoard(): Promise<void> {
    await reloadCanvas();
    await spaces.open();
    await spaces.option(spaceId).click();
    await waitForCanvas();
    await board.show('board');
  }

  before(async () => {
    await canvas.open();
    homeId = await homeSpaceId();
    spaceId = (await bridge.createSpace({ name: 'Gestes' })).id;

    perfId = (await bridge.createFolder({ spaceId, name: 'Perf' })).id;
    migrationsId = (await bridge.createFolder({ spaceId, name: 'Migrations' })).id;

    looseId = (await bridge.createNote(draft({ spaceId, title: 'Dump nocturne' }))).id;
    const filed = await bridge.createNote(draft({ spaceId, title: 'EXPLAIN lent sur join' }));
    filedId = filed.id;
    await bridge.fileNotes([filedId], perfId);

    await openBoard();
  });

  after(async () => {
    await board.show('date');
    await bridge.deleteSpace(spaceId, homeId);
    await reloadCanvas();
    await spaces.open();
    await spaces.allOption().click();
    await reloadCanvas();
  });

  /** Membership comes from the drop, and from nothing else. */
  it('files a card dragged into a zone', async () => {
    await board.dragCardInto(looseId, migrationsId);

    const view = await bridge.queryNotes(query({ spaceId, folderId: migrationsId }));
    expect(view.sections.flatMap((section) => section.notes).map((note) => note.title)).toContain(
      'Dump nocturne',
    );
  });

  /** ⚠️ A filed card flows inside its zone; only a loose one has a place of its own. */
  it('forgets where a filed card sat', async () => {
    const stillLoose = await browser.execute(
      (selector: string) => document.querySelector(selector) !== null,
      `[data-testid="board-loose-card"][data-note-id="${looseId}"]`,
    );

    expect(stillLoose).toBe(false);
  });

  /** The other direction of the same gesture: a drop on the background unfiles. */
  it('takes a card back out when it is dropped on the background', async () => {
    await board.drag(board.cardGrip(looseId), { dx: 120, dy: 620 });

    const view = await bridge.queryNotes(query({ spaceId, folderId: migrationsId }));
    expect(view.sections.flatMap((section) => section.notes).map((note) => note.title)).not.toContain(
      'Dump nocturne',
    );
  });

  it('remembers where the card was dropped across a restart of the front end', async () => {
    const dropped = await board.positionOf(looseId);
    expect(dropped.top).not.toBe('');

    await openBoard();

    expect(await board.positionOf(looseId)).toEqual(dropped);
  });

  it('moves a zone and keeps it there', async () => {
    const before = await board.frameOf(perfId);
    await board.drag(board.zoneGrip(perfId), { dx: 90, dy: 140 });

    const moved = await board.frameOf(perfId);
    expect(moved.left).not.toBe(before.left);
    expect(moved.width).toBe(before.width);

    await openBoard();
    expect(await board.frameOf(perfId)).toEqual(moved);
  });

  /** ⚠️ Moving a zone carries its notes: they flow inside it, so there is nothing to carry. */
  it('refiles nothing when a zone is moved', async () => {
    const view = await bridge.queryNotes(query({ spaceId, folderId: perfId }));
    expect(view.sections.flatMap((section) => section.notes).map((note) => note.id)).toEqual([filedId]);
  });

  /**
   * ⚠️ The rule Unreal's own comment box gets wrong: a frame that owns whatever it overlaps
   * silently refiles notes the day it is stretched. Membership comes from the drop.
   */
  it('captures and releases nothing when a zone is resized', async () => {
    const before = await board.frameOf(perfId);
    const looseBefore = (await bridge.queryNotes(query({ spaceId, folderId: perfId }))).matched;

    await board.drag(board.zoneResize(perfId), { dx: 260, dy: 200 });

    const resized = await board.frameOf(perfId);
    expect(Number.parseInt(resized.width, 10)).toBeGreaterThan(Number.parseInt(before.width, 10));
    expect(resized.left).toBe(before.left);
    expect((await bridge.queryNotes(query({ spaceId, folderId: perfId }))).matched).toBe(looseBefore);
  });

  it('creates a folder from a band drawn on the background', async () => {
    await board.drawZone({ x: 60, y: 900, width: 420, height: 320 });
    await board.nameZone('Reporting');

    const made = await eventually(
      async () => (await bridge.listFolders(spaceId)).find((folder) => folder.name === 'Reporting'),
      (folder) => folder !== undefined,
      'the drawn band to become a folder',
    );
    expect(made).toBeDefined();

    await openBoard();
    const frame = await board.frameOf(made!.id);
    expect(frame.left).toBe('60px');
    expect(frame.top).toBe('900px');
  });

  /**
   * ⚠️ The keyboard twin, and it is not optional: the linter requires it and the
   * application has held that line everywhere else. It is the path the selection bar
   * already offers, which is why the gesture adds to it rather than replacing it.
   */
  it('files a note with no pointer at all, from the selection bar', async () => {
    await canvas.check('EXPLAIN lent sur join');
    await selectionBar.fileInto(migrationsId);

    const view = await eventually(
      () => bridge.queryNotes(query({ spaceId, folderId: migrationsId })),
      (filed) =>
        filed.sections
          .flatMap((section) => section.notes)
          .some((note) => note.title === 'EXPLAIN lent sur join'),
      'the keyboard filing to land',
    );
    await selectionBar.clear();
    expect(view.sections.flatMap((section) => section.notes).map((note) => note.title)).toContain(
      'EXPLAIN lent sur join',
    );
  });

  it('takes it back out again with no pointer either', async () => {
    await canvas.check('EXPLAIN lent sur join');
    await selectionBar.fileInto(null);

    const view = await eventually(
      () => bridge.queryNotes(query({ spaceId, folderId: migrationsId })),
      (left) =>
        !left.sections
          .flatMap((section) => section.notes)
          .some((note) => note.title === 'EXPLAIN lent sur join'),
      'the keyboard unfiling to land',
    );
    await selectionBar.clear();
    expect(view.sections.flatMap((section) => section.notes).map((note) => note.title)).not.toContain(
      'EXPLAIN lent sur join',
    );
  });

  /**
   * ⚠️ A frame is computed once, on the board's first read, and never again — so a card
   * filed into a zone that was already full flowed out of sight behind its scrollbar.
   */
  /**
   * ⚠️ Membership was the one thing no overlay covered. The ghost the drag drew vanished
   * on `pointerup` and the card was drawn again where the last view had it — inside its
   * old zone, or loose at its old place — until the round trip landed (#282).
   */
  it('draws a card in its new home the moment it is let go of', async () => {
    const moving = (await bridge.createNote(draft({ spaceId, title: 'Bascule instantanee' }))).id;
    await openBoard();
    await eventually(
      () => board.holderOf(moving),
      (holder) => holder === null,
      'the new card to reach the background',
    );

    // ⚠️ No settling: this reads the board the frame after the drop, which is where the
    // card used to be drawn back at its old place. The unit specs are what pin it with
    // the view held still; here a fast round trip could answer the same thing honestly.
    await board.dropCardInto(moving, migrationsId);
    expect(await board.holderOf(moving)).toBe(migrationsId);

    // ⚠️ Let it land before the opposite gesture, or the two files race each other.
    await eventually(
      () => bridge.queryNotes(query({ spaceId, folderId: migrationsId })),
      (view) => view.sections.flatMap((section) => section.notes).some((note) => note.id === moving),
      'the filing to reach the database',
    );

    await board.drag(board.cardGrip(moving), { dx: 120, dy: 620 });
    expect(await board.holderOf(moving)).toBeNull();
    expect(
      await eventually(
        () => bridge.queryNotes(query({ spaceId, folderId: migrationsId })),
        (view) => !view.sections.flatMap((section) => section.notes).some((note) => note.id === moving),
        'the unfiling to reach the database',
      ),
    ).toBeTruthy();

    // ⚠️ Taken off the board again. This card was let go of at the place the scenario
    // above drops its own, and the last scenario in this file asserts that no loose card
    // covers another — a claim about the whole background, which this one would break.
    await bridge.deleteNote(moving);
    await openBoard();
  });

  it('makes room in a zone for the card filed into it', async () => {
    for (const title of ['Index manquant', 'Vacuum nocturne', 'Plan de requête']) {
      await bridge.fileNotes([(await bridge.createNote(draft({ spaceId, title }))).id], perfId);
    }
    await openBoard();

    expect(
      await eventually(
        () => board.zoneHoldsItsCards(perfId),
        (holds) => holds,
        'the zone to open far enough for what was filed into it',
      ),
    ).toBe(true);
  });

  /**
   * ⚠️ The report: a note captured from the clipboard was written under a card that was
   * already on the board. A note created now is the most recently updated, so it arrives
   * first in the list and used to be handed the seat its index gave it — seat zero, where
   * the board's first read had already put another card.
   */
  it('puts a note it has never placed on free ground', async () => {
    await bridge.createNote(draft({ spaceId, title: 'Collé du presse-papiers' }));
    await openBoard();

    const boxes = await eventually(
      () => board.looseBoxes(),
      (loose) => loose.some((card) => card.title === 'Collé du presse-papiers'),
      'the new card to reach the board',
    );

    const overlapping = boxes.flatMap((card, index) =>
      boxes
        .slice(index + 1)
        .filter(
          (other) =>
            card.left < other.right &&
            other.left < card.right &&
            card.top < other.bottom &&
            other.top < card.bottom,
        )
        .map((other) => [card.title, other.title]),
    );

    expect(boxes.length).toBeGreaterThan(1);
    expect(overlapping).toEqual([]);
  });

  /**
   * ⚠️ `columns_in` used to take a scrollbar off the width that `.zone-body` was not
   * showing. A zone dragged a little narrower than nominal still flowed two cards across
   * and was told it held one, so the next card filed in bought a whole extra row — a band
   * of empty board under the cards, which is what was reported (#284).
   */
  it('adds one row and not two to a zone dragged narrower than nominal', async () => {
    const rapports = (await bridge.createFolder({ spaceId, name: 'Rapports' })).id;
    for (const title of ['Coûts mensuels', 'Taux de conversion']) {
      await bridge.fileNotes([(await bridge.createNote(draft({ spaceId, title }))).id], rapports);
    }
    await openBoard();

    const nominal = Number.parseInt((await board.frameOf(rapports)).width, 10);
    await board.drag(board.zoneResize(rapports), { dx: -14, dy: 0 });
    const shaved = await eventually(
      () => storedFrame(rapports),
      (frame) => frame !== null && frame.width < nominal,
      'the narrower zone to reach the database',
    );

    const third = (await bridge.createNote(draft({ spaceId, title: 'Panier moyen' }))).id;
    await bridge.fileNotes([third], rapports);
    await openBoard();
    await eventually(
      () => board.zoneHoldsItsCards(rapports),
      (holds) => holds,
      'the zone to open far enough for what was filed into it',
    );

    // Two cards still fit across at this width, so three of them are two rows of 2 and 1.
    expect(shaved?.width).toBeGreaterThan(0);
    expect(await board.zoneRows(rapports)).toEqual([2, 1]);
    expect(await board.zoneSlack(rapports)).toBeLessThan(CARD_HEIGHT);
    // ⚠️ Guessed in Rust, so it has to be checked where it is drawn: one pixel over and
    // the body is short of its own rows, which costs a scrollbar and then a column.
    expect(await board.zoneHeaderHeight(rapports)).toBeLessThanOrEqual(ZONE_HEADER);
  });

  /**
   * Eleven cards in a zone was eleven clicks, and the zone already knows what it holds.
   *
   * ⚠️ It ticks what the zone is **showing**, dimmed cards included: the board dims rather
   * than narrowing, so a card the search filtered out of the date view is still filed here.
   */
  describe('selecting a whole folder', () => {
    it('ticks every card of a zone in one gesture', async () => {
      await openBoard();
      const held = (await bridge.queryNotes(query({ spaceId, folderId: perfId }))).matched;
      expect(held).toBeGreaterThan(1);

      await board.selectZoneNotes(perfId);

      expect(
        await eventually(
          () => selectionBar.count(),
          (count) => count.includes(String(held)),
          'the selection bar to name what the zone holds',
        ),
      ).toContain(String(held));
      await selectionBar.clear();
    });

    it('keeps a card the search has dimmed, because it is still in the folder', async () => {
      await canvas.search('Index manquant');
      const dimmed = await eventually(
        () => board.isDimmed('Vacuum nocturne'),
        (yes) => yes,
        'a card of the zone to be dimmed by the search',
      );
      expect(dimmed).toBe(true);

      await board.selectZoneNotes(perfId);

      const held = (await bridge.queryNotes(query({ spaceId, folderId: perfId }))).matched;
      expect(
        await eventually(
          () => selectionBar.count(),
          (count) => count.includes(String(held)),
          'the dimmed cards to be selected along with the rest',
        ),
      ).toContain(String(held));

      await selectionBar.clear();
      await canvas.clearSearch();
    });
  });

  /**
   * ⚠️ Last in the file: it rewrites every frame and every seat of the space, so any
   * scenario asserting a place of its own has to have run already.
   */
  describe('tidying it up', () => {
    /** ⚠️ The half worth a corner click: a zone sized by hand is the only manual work a
     *  board holds, and the frequent gesture must not be what overwrites it. */
    it('aligns the loose cards without touching a single zone', async () => {
      await openBoard();
      await board.drag(board.zoneGrip(perfId), { dx: 340, dy: 420 });
      const dragged = await eventually(
        () => storedFrame(perfId),
        (frame) => frame !== null && frame.x > 300,
        'the dragged zone to reach the database',
      );

      await board.align();

      // ⚠️ An assertion that nothing happened, so there is no condition to wait on: the
      // pause is deliberately a duration.
      await browser.pause(1500);
      expect(await storedFrame(perfId)).toEqual(dragged);
    });

    it('names what it will touch rather than warning about it', async () => {
      await $(testid('board-tidy-more')).click();

      expect(await board.reorganiseLabel()).toMatch(/d/);
      await browser.keys('Escape');
    });

    it('brings a zone dragged off into the distance back to its seat', async () => {
      await board.reorganise();

      const tidied = await eventually(
        () => storedFrame(perfId),
        (frame) => frame !== null && frame.x < 300,
        'the reorganisation to reach the database',
      );
      expect(tidied?.y).toBe(16);
    });

    /**
     * ⚠️ The result happens off screen otherwise. The pan is a native scroll nothing else
     * resets, so a board panned to the right lands everything at the top left and leaves
     * empty ground under a banner announcing success.
     */
    it('pans back to what it just wrote', async () => {
      await openBoard();
      await board.panTo(600, 200);
      await eventually(
        () => board.pan(),
        (at) => at.x > 0,
        'the board to be panned away from its origin',
      );

      await board.reorganise();

      expect(
        await eventually(
          () => board.pan(),
          (at) => at.x === 0 && at.y === 0,
          'the board to pan back to what the arrangement wrote',
        ),
      ).toEqual({ x: 0, y: 0 });
    });

    /**
     * ⚠️ Non-optional: it overwrites sizes chosen by hand, which dragging cannot undo.
     *
     * ⚠️ It drags a zone away first rather than leaning on the scenario above. The count
     * is computed on what actually **moved**, so reorganising a board already in order
     * opens no undo window at all — which is the point, and which made this read as a
     * missing bar when it was a correct refusal.
     */
    it('offers the previous arrangement back, and puts it back', async () => {
      await openBoard();
      await board.drag(board.zoneGrip(perfId), { dx: 300, dy: 380 });
      const dragged = await eventually(
        () => storedFrame(perfId),
        (frame) => frame !== null && frame.x > 300,
        'the dragged zone to reach the database',
      );

      await board.reorganise();
      await eventually(
        () => storedFrame(perfId),
        (frame) => frame !== null && frame.x < 300,
        'the reorganisation to reach the database',
      );

      await undoBar.bar().waitForExist({ timeout: 5_000 });
      await undoBar.restore();

      const restored = await eventually(
        () => storedFrame(perfId),
        (frame) => frame !== null && frame.x === dragged!.x,
        'the previous arrangement to come back',
      );
      expect(restored).toEqual(dragged);
    });
  });
});
