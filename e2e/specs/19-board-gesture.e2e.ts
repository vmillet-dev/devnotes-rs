import { browser, expect } from '@wdio/globals';

import { canvas } from '../pageobjects/canvas.page.js';
import { board, selectionBar, spaces } from '../pageobjects/overlays.page.js';
import { boxOf, eventually, reloadCanvas, testid, waitForCanvas } from '../support/app.js';
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
describe('Arranging the board', () => {
  let homeId = '';
  let spaceId = '';
  let perfId = '';
  let migrationsId = '';
  let looseId = '';
  let filedId = '';

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

  /**
   * ⚠️ The count used to be drawn above whichever loose card happened to be highest, so
   * it climbed over the zones as soon as one was dragged up — and off the top of the
   * board when the offset took it negative.
   */
  it('keeps the no-folder count in the corner while a card is dragged', async () => {
    const before = await boxOf(testid('board-loose-label'));

    await board.drag(board.cardGrip(looseId), { dx: 40, dy: -80 });

    expect(await boxOf(testid('board-loose-label'))).toEqual(before);
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
});
