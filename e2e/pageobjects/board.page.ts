import { $, browser } from '@wdio/globals';

import { readEach, setField, testid, waitForCanvas } from '../support/app.js';

/** The Date / Tableau switch and the board it draws. */
export const board = {
  option: (mode: 'date' | 'board') => $(testid(`view-${mode}`)),

  pressed: (mode: 'date' | 'board') => $(testid(`view-${mode}`)).getAttribute('aria-pressed'),

  isShowing: () => $(testid('board')).isExisting(),

  async waitForBoard(): Promise<void> {
    await $(testid('board')).waitForExist({ timeout: 15_000 });
    // Settled means the cards have arrived, not merely that the surface has.
    await browser.pause(400);
  },

  /** Idempotent: clicking the showing view again would change nothing but still re-render. */
  async show(mode: 'date' | 'board'): Promise<void> {
    if ((await board.pressed(mode)) === 'true') return;

    await board.option(mode).click();
    if (mode === 'board') {
      await board.waitForBoard();
    } else {
      await waitForCanvas();
    }
  },

  zoneNames: (): Promise<string[]> => readEach(testid('board-zone-open'), 'text'),

  /** Clicking a zone title is the descent. */
  async openZone(name: string): Promise<void> {
    await $(
      `${testid('board-zone')}[data-folder-id="${await board.folderId(name)}"] ${testid('board-zone-open')}`,
    ).click();
  },

  /**
   * ⚠️ Read in one call, like `canvas.titles()`: a round trip per card leaves a window in
   * which the board re-renders, and the list that comes back mixes two states.
   */
  zoneTitles(folderName: string): Promise<string[]> {
    return browser.execute(
      (zoneSelector: string, openSelector: string, cardTitle: string, wanted: string) =>
        [...document.querySelectorAll(zoneSelector)]
          .filter((zone) => (zone.querySelector(openSelector)?.textContent ?? '').trim() === wanted)
          .flatMap((zone) =>
            [...zone.querySelectorAll(cardTitle)].map((title) => (title.textContent ?? '').trim()),
          ),
      testid('board-zone'),
      testid('board-zone-open'),
      testid('note-card-title'),
      folderName,
    );
  },

  /** Whether every card in a zone is drawn inside the zone's own box, none cut off. */
  /**
   * How the browser actually flowed a zone: one entry per row, each the number of cards
   * on it. ⚠️ The claim Rust makes when it sizes a zone, read back off the screen — a
   * zone one pixel short of its rows shows a scrollbar, the scrollbar takes a slice of the
   * row, and two cards across silently become one (#284).
   */
  async zoneRows(folderId: string): Promise<number[]> {
    const tops = await browser.execute(
      (zoneSelector: string, cardSelector: string) => {
        const zone = document.querySelector(zoneSelector);
        if (!zone) throw new Error('no zone at ' + zoneSelector);

        return [...zone.querySelectorAll(cardSelector)].map((card) =>
          Math.round(card.getBoundingClientRect().top),
        );
      },
      `${testid('board-zone')}[data-folder-id="${folderId}"]`,
      testid('note-card'),
    );

    const rows = new Map<number, number>();
    for (const top of tops) {
      rows.set(top, (rows.get(top) ?? 0) + 1);
    }
    return [...rows.entries()].sort(([a], [b]) => a - b).map(([, count]) => count);
  },

  /** The header, whose height is the one number `board.rs` guesses rather than reads. */
  zoneHeaderHeight(folderId: string): Promise<number> {
    return browser.execute(
      (zoneSelector: string) => {
        const head = document.querySelector(zoneSelector)?.querySelector('.zone-head');
        if (!head) throw new Error('no zone head at ' + zoneSelector);

        return Math.ceil(head.getBoundingClientRect().height);
      },
      `${testid('board-zone')}[data-folder-id="${folderId}"]`,
    );
  },

  /**
   * The empty board left under the lowest card in a zone. ⚠️ A whole card’s worth of it
   * is the symptom of the count and the flow disagreeing: Rust made room for a row the
   * browser never used (#284).
   */
  zoneSlack(folderId: string): Promise<number> {
    return browser.execute(
      (zoneSelector: string, cardSelector: string) => {
        const zone = document.querySelector(zoneSelector);
        if (!zone) throw new Error('no zone at ' + zoneSelector);

        const box = zone.getBoundingClientRect();
        const lowest = [...zone.querySelectorAll(cardSelector)].reduce(
          (bottom, card) => Math.max(bottom, card.getBoundingClientRect().bottom),
          box.top,
        );
        return Math.round(box.bottom - lowest);
      },
      `${testid('board-zone')}[data-folder-id="${folderId}"]`,
      testid('note-card'),
    );
  },

  zoneHoldsItsCards(folderId: string): Promise<boolean> {
    return browser.execute(
      (zoneSelector: string, cardSelector: string) => {
        const zone = document.querySelector(zoneSelector);
        if (!zone) throw new Error('no zone at ' + zoneSelector);

        const box = zone.getBoundingClientRect();
        return [...zone.querySelectorAll(cardSelector)].every(
          (card) => card.getBoundingClientRect().bottom <= box.bottom + 1,
        );
      },
      `${testid('board-zone')}[data-folder-id="${folderId}"]`,
      testid('note-card'),
    );
  },

  looseTitles: (): Promise<string[]> =>
    readEach(testid('board-loose-card'), 'text', testid('note-card-title')),

  /** Where the loose cards actually are, so a spec can say whether two share a place. */
  looseBoxes(): Promise<{ title: string; left: number; top: number; right: number; bottom: number }[]> {
    return browser.execute(
      (selector: string, titleSelector: string) =>
        [...document.querySelectorAll<HTMLElement>(selector)].map((card) => {
          const box = card.getBoundingClientRect();
          return {
            title: (card.querySelector(titleSelector)?.textContent ?? '').trim(),
            left: box.left,
            top: box.top,
            right: box.right,
            bottom: box.bottom,
          };
        }),
      testid('board-loose-card'),
      testid('note-card-title'),
    );
  },

  async zoneCard(folderName: string, title: string) {
    const zone = $(`${testid('board-zone')}[data-folder-id="${await board.folderId(folderName)}"]`);
    const id = await browser.execute(
      (zoneSelector: string, cardSelector: string, titleSelector: string, wanted: string) =>
        [...(document.querySelector(zoneSelector)?.querySelectorAll(cardSelector) ?? [])]
          .find((card) => (card.querySelector(titleSelector)?.textContent ?? '').trim() === wanted)
          ?.getAttribute('data-note-id') ?? null,
      `${testid('board-zone')}[data-folder-id="${await board.folderId(folderName)}"]`,
      testid('note-card'),
      testid('note-card-title'),
      title,
    );

    if (id === null) {
      throw new Error(`no card titled "${title}" in zone "${folderName}"`);
    }
    return zone.$(`${testid('note-card')}[data-note-id="${id}"]`);
  },

  async folderId(name: string): Promise<string> {
    const id = await browser.execute(
      (zoneSelector: string, openSelector: string, wanted: string) =>
        [...document.querySelectorAll(zoneSelector)]
          .find((zone) => (zone.querySelector(openSelector)?.textContent ?? '').trim() === wanted)
          ?.getAttribute('data-folder-id') ?? null,
      testid('board-zone'),
      testid('board-zone-open'),
      name,
    );

    if (id === null) {
      throw new Error(`no zone named "${name}" on the board`);
    }
    return id;
  },

  /** The frames as the board actually placed them, which is what a restart must reproduce. */
  zoneFrames(): Promise<{ left: string; top: string; width: string }[]> {
    return browser.execute(
      (selector: string) =>
        [...document.querySelectorAll<HTMLElement>(selector)].map((zone) => ({
          left: zone.style.left,
          top: zone.style.top,
          width: zone.style.width,
        })),
      testid('board-zone'),
    );
  },

  /** Which zone a card is drawn in right now, or `null` for the free background. */
  holderOf(noteId: string): Promise<string | null> {
    return browser.execute(
      (cardSelector: string, zoneSelector: string) => {
        const card = document.querySelector(cardSelector);
        return card === null ? null : (card.closest(zoneSelector)?.getAttribute('data-folder-id') ?? null);
      },
      `${testid('note-card')}[data-note-id="${noteId}"]`,
      testid('board-zone'),
    );
  },

  /** The frame of one zone as the board actually placed it. */
  frameOf(folderId: string): Promise<{ left: string; top: string; width: string; height: string }> {
    return browser.execute(
      (selector: string) => {
        const zone = document.querySelector<HTMLElement>(selector);
        if (!zone) throw new Error(`no zone ${selector}`);
        return {
          left: zone.style.left,
          top: zone.style.top,
          width: zone.style.width,
          height: zone.style.height,
        };
      },
      `${testid('board-zone')}[data-folder-id="${folderId}"]`,
    );
  },

  /** Where a loose card sits, which is the only place a card has one. */
  positionOf(noteId: string): Promise<{ left: string; top: string }> {
    return browser.execute(
      (selector: string) => {
        const card = document.querySelector<HTMLElement>(selector);
        if (!card) throw new Error(`no loose card ${selector}`);
        return { left: card.style.left, top: card.style.top };
      },
      `${testid('board-loose-card')}[data-note-id="${noteId}"]`,
    );
  },

  async nameZone(name: string): Promise<void> {
    await $(testid('folder-name-input')).waitForExist({ timeout: 5_000 });
    await setField(testid('folder-name-input'), name);
    await $(testid('folder-name-submit')).click();
    await browser.pause(800);
  },

  /** Ticks every card of a zone, dimmed ones included: the board dims, it does not narrow. */
  async selectZoneNotes(folderId: string): Promise<void> {
    await $(`${testid('board-zone')}[data-folder-id="${folderId}"] ${testid('board-zone-menu')}`).click();
    await $(testid('folder-select-notes')).click();
  },

  /** The corner click: the loose cards alone, nothing anybody sized by hand. */
  align: () => $(testid('board-tidy')).click(),

  /** A notch further: the zones as well, which is why it is behind the chevron. */
  async reorganise(): Promise<void> {
    await $(testid('board-tidy-more')).click();
    await $(testid('board-tidy-everything')).click();
  },

  /** What the label offers to touch, which is the count and not a warning. */
  reorganiseLabel: () => $(testid('board-tidy-everything')).getText(),

  /** ⚠️ Where the board is panned to. An arrangement that lands its result outside this
   *  is indistinguishable from an erasure. */
  pan: (): Promise<{ x: number; y: number }> =>
    browser.execute(() => {
      const board = document.querySelector('.board');
      return { x: board?.scrollLeft ?? 0, y: board?.scrollTop ?? 0 };
    }),

  panTo: (x: number, y: number): Promise<void> =>
    browser.execute(
      (left: number, top: number) => {
        document.querySelector('.board')?.scrollTo(left, top);
      },
      x,
      y,
    ),

  /** ⚠️ Dimmed, never dropped: the card is still there, it has only stopped shouting. */
  isDimmed(title: string): Promise<boolean> {
    return browser.execute(
      (cardSelector: string, titleSelector: string, wanted: string) =>
        [...document.querySelectorAll(cardSelector)]
          .find((card) => (card.querySelector(titleSelector)?.textContent ?? '').trim() === wanted)
          ?.classList.contains('dimmed') ?? false,
      '.zone-card, .loose-card',
      testid('note-card-title'),
      title,
    );
  },
};
