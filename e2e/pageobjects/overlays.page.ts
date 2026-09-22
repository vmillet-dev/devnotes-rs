import { $, $$, browser } from '@wdio/globals';

import {
  confirmTwice,
  readEach,
  setField,
  pickChoice,
  submitFormOf,
  testid,
  waitForCanvas,
} from '../support/app.js';

/** The rail itself: shown or hidden, and remembered across launches. */
export const rail = {
  isShowing: () => $(testid('library-rail')).isExisting(),

  toggle: () => $(testid('library-rail-toggle')).click(),

  async show(): Promise<void> {
    if (await rail.isShowing()) return;

    await rail.toggle();
    await $(testid('library-rail')).waitForExist({ timeout: 5_000 });
  },

  async hide(): Promise<void> {
    if (!(await rail.isShowing())) return;

    await rail.toggle();
    await $(testid('library-rail')).waitForExist({ reverse: true, timeout: 5_000 });
  },

  /** Measured rather than read off the preference: the rail has to actually be that wide. */
  width: (): Promise<number> =>
    browser.execute(
      (selector: string) => Math.round(document.querySelector(selector)?.getBoundingClientRect().width ?? 0),
      testid('library-rail'),
    ),

  /**
   * Two arrows on the edge, the keyboard twin of dragging it. ⚠️ Focused from script: the
   * edge swallows its own `pointerdown` to start a drag, and that is what would focus it.
   */
  async widen(): Promise<void> {
    await browser.execute(
      (selector: string) => (document.querySelector(selector) as HTMLElement | null)?.focus(),
      testid('library-rail-edge'),
    );
    await browser.keys(['ArrowRight', 'ArrowRight']);
  },

  /** The same edge the other way, far enough to reach the floor from any width. */
  async narrow(): Promise<void> {
    await browser.execute(
      (selector: string) => (document.querySelector(selector) as HTMLElement | null)?.focus(),
      testid('library-rail-edge'),
    );
    await browser.keys(Array.from({ length: 30 }, () => 'ArrowLeft'));
  },

  /**
   * How far a control sticks out past the rail's right edge. The panels a ⋯ opens are
   * projected inline between the rows, so one holding a minimum of its own would spill
   * over the edge instead of following it.
   */
  overflowOf: (selector: string): Promise<number> =>
    browser.execute(
      (railSelector: string, controlSelector: string) => {
        const edge = document.querySelector(railSelector)?.getBoundingClientRect().right;
        const control = document.querySelector(controlSelector)?.getBoundingClientRect().right;
        return edge === undefined || control === undefined ? -1 : Math.round(Math.max(0, control - edge));
      },
      testid('library-rail'),
      selector,
    ),
};

/**
 * The spaces as the library rail draws them: every space is a row, and the ⋯ beside one
 * opens the panel that pins, renames and deletes it.
 */
export const spaces = {
  /** The rail holds the rows, so "open" is "make sure the rail is showing". */
  async open(): Promise<void> {
    await rail.show();
  },

  /** Closes whatever panel a row's ⋯ left open; the rail itself stays. */
  async close(): Promise<void> {
    const panel = $(`${testid('space-edit')}[aria-expanded="true"]`);
    if (await panel.isExisting()) {
      await panel.click();
    }
  },

  /** The row the canvas is showing — the only thing on screen saying which space that is. */
  label: () => $('[data-testid^="space-option"][aria-current="true"]').getText(),
  option: (id: string) => $(`${testid('space-option')}[data-space-id="${id}"]`),

  /** `null` is "all spaces", and it is a choice rather than a loading state. */
  allOption: () => $(testid('space-option-all')),

  names: (): Promise<string[]> => readEach(testid('space-option'), 'text'),

  async create(name: string): Promise<void> {
    await $(testid('space-create-open')).click();
    // The form is revealed by that click: the field does not exist until it lands.
    await setField(testid('space-create-input'), name);
    await $(testid('space-create-submit')).click();
  },

  /** A space's folders are drawn under it; this is what folds them away. */
  async collapse(id: string): Promise<void> {
    await $(`${testid('space-twisty')}[data-space-id="${id}"]`).click();
  },

  async rename(id: string, into: string): Promise<void> {
    await $(`${testid('space-edit')}[data-space-id="${id}"]`).click();
    await setField(testid('space-rename-input'), into);
    await $(testid('space-rename-submit')).click();
  },

  /**
   * From the same panel as the rename and the delete, which the ⋯ opens. ⚠️ Closes behind
   * itself: a panel left open covers the rows the next caller is looking for.
   */
  async togglePin(id: string): Promise<void> {
    await $(`${testid('space-edit')}[data-space-id="${id}"]`).click();
    await $(testid('space-pin')).click();
    await spaces.close();
  },

  /** ⚠️ The refuge is mandatory: the space leaves with its notes if nobody takes them in. */
  async remove(id: string, refugeId: string): Promise<void> {
    await $(`${testid('space-edit')}[data-space-id="${id}"]`).click();
    await pickChoice('space-move-target', refugeId);
    await confirmTwice($(testid('space-delete')));
  },

  deleteBlocked: () => $(testid('space-delete-blocked')),
};

/** The folders, drawn in the rail under the space that holds them. */
export const folders = {
  async open(): Promise<void> {
    await rail.show();
  },

  /** Closes whatever panel a row's ⋯ left open; the rail itself stays. */
  async close(): Promise<void> {
    const panel = $(`${testid('folder-edit')}[aria-expanded="true"]`);
    if (await panel.isExisting()) {
      await panel.click();
    }
  },

  label: () => $(`${testid('folder-option')}[aria-current="true"]`).getText(),
  option: (id: string) => $(`${testid('folder-option')}[data-folder-id="${id}"]`),

  names: (): Promise<string[]> => readEach(testid('folder-option'), 'text'),

  async create(name: string): Promise<void> {
    await $(testid('folder-create-open')).click();
    // The form is revealed by that click: the field does not exist until it lands.
    await setField(testid('folder-create-input'), name);
    await $(testid('folder-create-submit')).click();
  },

  async rename(id: string, into: string): Promise<void> {
    await $(`${testid('folder-edit')}[data-folder-id="${id}"]`).click();
    await setField(testid('folder-rename-input'), into);
    await $(testid('folder-rename-submit')).click();
  },

  /** ⚠️ Closes behind itself: a panel left open covers the rows under it. */
  async recolour(id: string, colour: string): Promise<void> {
    await $(`${testid('folder-edit')}[data-folder-id="${id}"]`).click();
    await $(`${testid('folder-colour')}[data-colour="${colour}"]`).click();
    await folders.close();
  },

  /** ⚠️ No refuge to choose, unlike a space: the notes simply come out loose. */
  async remove(id: string): Promise<void> {
    await $(`${testid('folder-edit')}[data-folder-id="${id}"]`).click();
    await confirmTwice($(testid('folder-delete')));
  },
};

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

  /**
   * A pointer drag, dispatched from inside the page.
   *
   * ⚠️ Synthetic, like every other input this suite sends: the embedded driver drops
   * WebDriver actions. So this proves the wiring — grip to store to command to database —
   * and not the WebView's own pointer capture, which the unit specs cover instead.
   */
  /** Drags a loose card into a zone and waits for the write to have landed. */
  async dragCardInto(noteId: string, folderId: string): Promise<void> {
    await board.dropCardInto(noteId, folderId);
    // ⚠️ A duration, deliberately: what the callers of this one go on to read is the
    // database, and no condition in the page says the debounced write has reached it.
    await browser.pause(1200);
  },

  /**
   * The gesture alone. ⚠️ Nothing is waited on: a spec using this is asking what the
   * board draws **before** the round trip, which is what the staged overlay is for.
   */
  async dropCardInto(noteId: string, folderId: string): Promise<void> {
    await browser.execute(
      (cardSelector: string, zoneSelector: string, gripSelector: string) => {
        const card = document.querySelector(cardSelector);
        const zone = document.querySelector(zoneSelector);
        const grip = document.querySelector(gripSelector);
        const surface = document.querySelector('[data-testid="board-surface"]');
        if (!card || !zone || !grip || !surface) throw new Error('nothing to drag');

        const cardBox = card.getBoundingClientRect();
        const zoneBox = zone.getBoundingClientRect();
        const gripBox = grip.getBoundingClientRect();
        // Where the card's own corner has to end up for the drop to land in the zone.
        const dx = zoneBox.left + 60 - cardBox.left;
        const dy = zoneBox.top + 80 - cardBox.top;

        const from = { x: gripBox.left + gripBox.width / 2, y: gripBox.top + gripBox.height / 2 };
        const send = (target: Element, type: string, x: number, y: number) =>
          target.dispatchEvent(
            new PointerEvent(type, {
              bubbles: true,
              cancelable: true,
              clientX: x,
              clientY: y,
              button: 0,
              pointerId: 1,
            }),
          );

        send(grip, 'pointerdown', from.x, from.y);
        send(surface, 'pointermove', from.x + dx, from.y + dy);
        send(surface, 'pointerup', from.x + dx, from.y + dy);
      },
      `${testid('board-loose-card')}[data-note-id="${noteId}"]`,
      `${testid('board-zone')}[data-folder-id="${folderId}"]`,
      board.cardGrip(noteId),
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

  async drag(gripSelector: string, to: { dx: number; dy: number }): Promise<void> {
    await browser.execute(
      (selector: string, dx: number, dy: number) => {
        const grip = document.querySelector(selector);
        const surface = document.querySelector('[data-testid="board-surface"]');
        if (!grip || !surface) throw new Error(`nothing to drag at ${selector}`);

        const box = grip.getBoundingClientRect();
        const from = { x: box.left + box.width / 2, y: box.top + box.height / 2 };
        const send = (target: Element, type: string, x: number, y: number) =>
          target.dispatchEvent(
            new PointerEvent(type, {
              bubbles: true,
              cancelable: true,
              clientX: x,
              clientY: y,
              button: 0,
              pointerId: 1,
            }),
          );

        send(grip, 'pointerdown', from.x, from.y);
        send(surface, 'pointermove', from.x + dx, from.y + dy);
        send(surface, 'pointerup', from.x + dx, from.y + dy);
      },
      gripSelector,
      to.dx,
      to.dy,
    );
    // Past the layout debounce, so the assertion reads what was actually written.
    await browser.pause(1200);
  },

  /**
   * Sweeps a selection band with the **right** button, in surface coordinates.
   *
   * ⚠️ The right one because the left is taken: dragging the background draws a folder,
   * and that gesture does not move.
   */
  async bandSelect(at: { x: number; y: number; width: number; height: number }): Promise<void> {
    await browser.execute(
      (x: number, y: number, width: number, height: number) => {
        const surface = document.querySelector('[data-testid="board-surface"]');
        if (!surface) throw new Error('no board surface');

        const box = surface.getBoundingClientRect();
        const send = (type: string, cx: number, cy: number) =>
          surface.dispatchEvent(
            new PointerEvent(type, {
              bubbles: true,
              cancelable: true,
              clientX: cx,
              clientY: cy,
              button: 2,
              pointerId: 1,
            }),
          );

        send('pointerdown', box.left + x, box.top + y);
        send('pointermove', box.left + x + width, box.top + y + height);
        send('pointerup', box.left + x + width, box.top + y + height);
      },
      at.x,
      at.y,
      at.width,
      at.height,
    );
  },

  /** Whether the browser's own menu would have opened where a sweep ends. */
  contextMenuRefused: (): Promise<boolean> =>
    browser.execute(() => {
      const surface = document.querySelector('[data-testid="board-surface"]');
      if (!surface) throw new Error('no board surface');
      const event = new MouseEvent('contextmenu', { bubbles: true, cancelable: true });
      surface.dispatchEvent(event);
      return event.defaultPrevented;
    }),

  /** Draws a band on the empty background, in surface coordinates. */
  async drawZone(at: { x: number; y: number; width: number; height: number }): Promise<void> {
    await browser.execute(
      (x: number, y: number, width: number, height: number) => {
        const surface = document.querySelector('[data-testid="board-surface"]');
        if (!surface) throw new Error('no board surface');

        const box = surface.getBoundingClientRect();
        const send = (type: string, cx: number, cy: number) =>
          surface.dispatchEvent(
            new PointerEvent(type, {
              bubbles: true,
              cancelable: true,
              clientX: cx,
              clientY: cy,
              button: 0,
              pointerId: 1,
            }),
          );

        send('pointerdown', box.left + x, box.top + y);
        send('pointermove', box.left + x + width, box.top + y + height);
        send('pointerup', box.left + x + width, box.top + y + height);
      },
      at.x,
      at.y,
      at.width,
      at.height,
    );
    await browser.pause(400);
  },

  /** ⚠️ The card itself: the whole of it is the handle, there is no grip any more. */
  cardGrip: (noteId: string) => `[data-note-id="${noteId}"] ${testid('note-card-title')}`,
  zoneGrip: (folderId: string) =>
    `${testid('board-zone')}[data-folder-id="${folderId}"] ${testid('board-zone-grip')}`,
  zoneResize: (folderId: string) =>
    `${testid('board-zone')}[data-folder-id="${folderId}"] ${testid('board-zone-resize')}`,

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

/** The breadcrumb, which replaces the switchers while a folder is open. */
export const crumb = {
  isShowing: () => $(testid('folder-breadcrumb')).isExisting(),

  name: () => $(testid('folder-breadcrumb-name')).getText(),

  back: () => $(testid('folder-breadcrumb-back')).click(),

  swatchClass: () =>
    browser.execute(
      (selector: string) => document.querySelector(selector)?.className ?? '',
      `${testid('folder-breadcrumb')} ~ * .crumb-swatch, ${testid('folder-breadcrumb')} .crumb-swatch`,
    ),

  /** ⚠️ Opens behind the ⋯, which is the only way to the folder's own actions from here. */
  async openMenu(): Promise<void> {
    if (!(await $(testid('folder-breadcrumb-panel')).isExisting())) {
      await $(testid('folder-breadcrumb-menu')).click();
      await $(testid('folder-breadcrumb-panel')).waitForExist({ timeout: 5_000 });
    }
  },

  async rename(into: string): Promise<void> {
    await crumb.openMenu();
    await setField(testid('folder-rename-input'), into);
    await $(testid('folder-rename-submit')).click();
  },

  /** ⚠️ No refuge to choose, unlike a space: the notes come out loose. */
  async remove(): Promise<void> {
    await crumb.openMenu();
    await confirmTwice($(testid('folder-delete')));
  },
};

export const trash = {
  async open(): Promise<void> {
    await $(testid('trash-open')).click();
    await $(testid('trash-close')).waitForExist({ timeout: 10_000 });
  },

  rows: () => $$(testid('trash-row')),

  titles: (): Promise<string[]> => readEach(testid('trash-row'), 'text', testid('trash-row-title')),

  async restore(title: string): Promise<void> {
    const row = await trash.rowWithTitle(title);
    await row.$(testid('trash-restore')).click();
  },

  async purge(title: string): Promise<void> {
    const row = await trash.rowWithTitle(title);
    await confirmTwice(row.$(testid('trash-purge')));
  },

  /**
   * ⚠️ **Not** a second click on the same button, unlike the rest: nothing puts these notes
   * back, so the trigger is replaced by a sentence saying how many and a separate confirm.
   */
  async empty(): Promise<void> {
    await $(testid('trash-empty')).click();
    await $(testid('trash-empty-warning')).waitForExist({ timeout: 5_000 });
    await $(testid('trash-empty-confirm')).click();
  },

  /** Matched in one call and used as a selector, like `canvas.cardWithTitle`. */
  async rowWithTitle(title: string) {
    // Waited for, not read once: a single read that lands early reports a row missing
    // that is merely late.
    await browser.waitUntil(async () => (await trash.titles()).includes(title), {
      timeout: 10_000,
      timeoutMsg: `no trash row titled "${title}" ever appeared`,
    });

    const titles = await trash.titles();
    const index = titles.indexOf(title);
    if (index < 0) {
      throw new Error(`no trash row titled "${title}" — found ${JSON.stringify(titles)}`);
    }

    const ids = await readEach(testid('trash-row'), '@data-note-id');
    const id = ids[index];
    if (!id) {
      throw new Error(`the trash row titled "${title}" carries no id`);
    }

    return $(`${testid('trash-row')}[data-note-id="${id}"]`);
  },

  close: () => $(testid('trash-close')).click(),
  emptyState: () => $(testid('trash-empty-state')),
};

export const undoBar = {
  bar: () => $(testid('undo-bar')),
  restore: () => $(testid('undo-restore')).click(),
  dismiss: () => $(testid('undo-dismiss')).click(),
};

export const palette = {
  input: () => $(testid('palette-input')),
  options: () => $$(testid('palette-option')),
  createRow: () => $(testid('palette-create')),
  empty: () => $(testid('palette-empty')),

  titles: (): Promise<string[]> => readEach(testid('palette-option'), 'text'),

  /** A click on the row opens the note; the ⧉ beside it is the paste path. */
  openRow: (index = 0) => $$(testid('palette-open'))[index],
  copyRow: (index = 0) => $$(testid('palette-copy'))[index],

  /** Which row is current, read the way assistive technology reads it. */
  current: () => $(testid('palette-input')).getAttribute('aria-activedescendant'),

  /** What the keyboard is on — the field, and only ever the field. */
  focused: () => browser.execute(() => document.activeElement?.getAttribute('data-testid') ?? null),

  async type(text: string): Promise<void> {
    await $(testid('palette-input')).setValue(text);
    await browser.pause(400);
  },
};

export const fieldsForm = {
  form: () => $(testid('placeholder-form')),

  /** ⚠️ Scoped to the form: `placeholder-input` is the same hook in the editor's panel. */
  field: (name: string) =>
    $(testid('placeholder-form')).$(`${testid('placeholder-input')}[data-field="${name}"]`),

  submit: () => $(testid('placeholder-submit')).click(),

  /** ⚠️ Copies and dismisses: there is no form left to cancel afterwards. */
  copyRaw: () => $(testid('placeholder-copy-raw')).click(),

  cancel: () => $(testid('placeholder-cancel')).click(),
};

/** Reached from the end of the tag rail: it is a view on the notes, not a File entry. */
export const tagManager = {
  tag: (tag: string) => $(`${testid('tag-item')}[data-tag="${tag}"]`),
  select: (tag: string) => $(`${testid('tag-item')}[data-tag="${tag}"]`).click(),

  isSelected: async (tag: string) =>
    (await $(`${testid('tag-item')}[data-tag="${tag}"]`).getAttribute('aria-pressed')) === 'true',

  tags: (): Promise<string[]> => readEach(testid('tag-item'), '@data-tag'),

  async setTarget(value: string): Promise<void> {
    const field = $(testid('tag-target'));
    await field.click();
    await field.setValue(value);
  },

  /** Submits the form rather than clicking, so the `type="submit"` path is the one taken. */
  propose: () => submitFormOf(testid('tag-target')),

  proposeDelete: () => $(testid('tag-delete')).click(),

  confirmation: () => $(testid('tag-confirm')),

  confirm: () => $(testid('tag-confirm-apply')).click(),

  cancel: () => $(testid('tag-confirm-cancel')).click(),

  isApplyDisabled: async () => (await $(testid('tag-apply')).getAttribute('aria-disabled')) === 'true',

  /** Nothing corpus-wide writes without passing through the confirmation. */
  async apply(): Promise<void> {
    await submitFormOf(testid('tag-target'));
    await $(testid('tag-confirm-apply')).waitForDisplayed({ timeout: 10_000 });
    await $(testid('tag-confirm-apply')).click();
  },

  async delete(): Promise<void> {
    await $(testid('tag-delete')).click();
    await $(testid('tag-confirm-apply')).waitForDisplayed({ timeout: 10_000 });
    await $(testid('tag-confirm-apply')).click();
  },

  close: () => $(testid('tag-manager-close')).click(),
};

/** Shown only while at least one card is ticked. */
export const selectionBar = {
  bar: () => $(testid('selection-bar')),
  count: () => $(testid('selection-count')).getText(),
  clear: () => $(testid('selection-clear')).click(),
  copy: () => $(testid('selection-copy')).click(),

  moveTo: (spaceId: string) => pickChoice('selection-move', spaceId),

  /** The same control both ways: the way out is an entry of the same menu. */
  fileInto: (folderId: string | null) => pickChoice('selection-file', folderId ?? '__unfile__'),

  async tag(tag: string): Promise<void> {
    const field = $(testid('selection-tag'));
    await field.click();
    await field.setValue(tag);
    await submitFormOf(testid('selection-tag'));
  },

  async delete(): Promise<void> {
    const button = $(testid('selection-delete'));
    await button.click();
    await button.click();
  },
};
