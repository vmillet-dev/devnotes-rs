import { browser } from '@wdio/globals';

import { testid } from '../support/app.js';

/**
 * The board's pointer gestures, dispatched from inside the page.
 *
 * Synthetic, like every other input this suite sends: the embedded driver drops
 * WebDriver actions. So this proves the wiring — grip to store to command to database —
 * and not the WebView's own pointer capture, which the unit specs cover instead.
 */
export const gestures = {
  /** Drags a loose card into a zone and waits for the write to have landed. */
  async dragCardInto(noteId: string, folderId: string): Promise<void> {
    await gestures.dropCardInto(noteId, folderId);
    // A duration, deliberately: what the callers of this one go on to read is the
    // database, and no condition in the page says the debounced write has reached it.
    await browser.pause(1200);
  },

  /**
   * The gesture alone. Nothing is waited on: a spec using this is asking what the
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
      gestures.cardGrip(noteId),
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
   * The right one because the left is taken: dragging the background draws a folder,
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

  /**
   * Sweeps a band over empty ground, then asks for a menu **off the board**, twice: what
   * the page answers each time, in order. One call, so nothing can press in between.
   */
  menusAfterSweep: (offBoard: string): Promise<boolean[]> =>
    browser.execute((selector: string) => {
      const surface = document.querySelector('[data-testid="board-surface"]');
      const outside = document.querySelector(selector);
      if (!surface || !outside) throw new Error('no board surface, or nothing at ' + selector);

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
      send('pointerdown', box.left + 20, box.top + 3000);
      send('pointermove', box.left + 220, box.top + 3200);
      send('pointerup', box.left + 220, box.top + 3200);

      return [0, 1].map(() => {
        const menu = new MouseEvent('contextmenu', { bubbles: true, cancelable: true });
        outside.dispatchEvent(menu);
        return menu.defaultPrevented;
      });
    }, offBoard),

  /**
   * The computed fill of the band a button draws, read **while** it is drawn, then thrown
   * away with `pointercancel` so nothing is written. Computed, not the class: a
   * declaration the browser dropped as invalid leaves the class on and the band empty.
   */
  // `await` flattens what `execute` types as a promise of the page's own promise.
  bandFill: async (button: 0 | 2): Promise<string> =>
    await browser.execute((pressed: number) => {
      const surface = document.querySelector('[data-testid="board-surface"]');
      if (!surface) throw new Error('no board surface');

      const box = surface.getBoundingClientRect();
      const send = (type: string, x: number, y: number) =>
        surface.dispatchEvent(
          new PointerEvent(type, {
            bubbles: true,
            cancelable: true,
            clientX: x,
            clientY: y,
            button: pressed,
            pointerId: 1,
          }),
        );
      send('pointerdown', box.left + 40, box.top + 3000);
      send('pointermove', box.left + 300, box.top + 3200);

      // Zoneless: the band is drawn on the next change detection, not by the dispatch.
      return new Promise<string>((resolve) => {
        requestAnimationFrame(() =>
          requestAnimationFrame(() => {
            const band = document.querySelector(
              pressed === 2 ? '[data-testid="board-select-band"]' : '[data-testid="board-draw-band"]',
            );
            resolve(band ? getComputedStyle(band).backgroundColor : 'no band');
            send('pointercancel', box.left + 300, box.top + 3200);
          }),
        );
      });
    }, button),

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

  /** The card itself: the whole of it is the handle. */
  cardGrip: (noteId: string) => `[data-note-id="${noteId}"] ${testid('note-card-title')}`,
  zoneGrip: (folderId: string) =>
    `${testid('board-zone')}[data-folder-id="${folderId}"] ${testid('board-zone-grip')}`,
  zoneResize: (folderId: string) =>
    `${testid('board-zone')}[data-folder-id="${folderId}"] ${testid('board-zone-resize')}`,
};
