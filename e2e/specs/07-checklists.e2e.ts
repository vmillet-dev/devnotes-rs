import { browser, expect } from '@wdio/globals';

import { canvas } from '../pageobjects/canvas.page.js';
import { editor } from '../pageobjects/editor.page.js';
import { banners } from '../pageobjects/titlebar.page.js';
import { clipboardText, eventually, press, reloadCanvas, testid } from '../support/app.js';
import { bridge, draft, homeSpaceId, query } from '../support/bridge.js';

/**
 * `note_items` is keyed by position, so every write replaces the whole list. Reordering
 * is pointer events plus `Alt+↑/↓`, because HTML5 drag and drop receives nothing in this
 * WebView — the keyboard path is the one a test can drive.
 */
describe('Todo lists', () => {
  const title = 'Release checklist';

  before(canvas.open);

  async function reread() {
    const view = await bridge.queryNotes(query({ search: title }));
    return view.sections[0]?.notes[0];
  }

  it('creates a note of the checklist kind', async () => {
    await canvas.createChecklist();
    await editor.setTitle(title);
    await editor.addItem('Tag the release');
    await editor.close();

    await canvas.waitForCard(title);
    expect((await reread())?.kind).toBe('checklist');
  });

  it('keeps the items in the order they were added', async () => {
    await canvas.openNote(title);
    await editor.addItem('Write the changelog');
    await editor.addItem('Publish the binaries');
    await editor.close();

    expect(((await reread())?.items ?? []).map((item) => item.text)).toEqual([
      'Tag the release',
      'Write the changelog',
      'Publish the binaries',
    ]);
  });

  it('ticks an item from the editor', async () => {
    await canvas.openNote(title);
    await editor.toggleItem(0);
    expect(await editor.itemChecks()).toEqual([true, false, false]);
    await editor.close();

    expect((await reread())?.items?.[0]?.done).toBe(true);
  });

  it('ticks an item from the card, without opening it', async () => {
    // The items sit on a layer above the card button, which is why they can be clicked
    // at all — a `<div>` inside a `<button>` would be invalid HTML.
    const card = await canvas.cardWithTitle(title);
    // The rows are what is left to do, so the first one is not the item the editor ticked.
    await card.$(testid('note-card-item')).click();

    const done = await eventually(
      async () => ((await reread())?.items ?? []).map((item) => item.done),
      (state) => state[1] === true,
      'the row the card drew to come back ticked',
    );
    expect(done).toEqual([true, true, false]);
  });

  it('offers a drag handle that says what it moves', async () => {
    await canvas.openNote(title);
    // Asserted, not dragged — see `editor.grip`.
    const grip = await editor.grip(0);
    expect(await grip.isExisting()).toBe(true);
    expect(await grip.getAttribute('aria-label')).toContain('Tag the release');
    await editor.close();
  });

  it('reorders with Alt+arrow, the twin of the pointer drag', async () => {
    await canvas.openNote(title);
    await editor.moveItemUp(1);
    await editor.close();

    expect(((await reread())?.items ?? []).map((item) => item.text)).toEqual([
      'Write the changelog',
      'Tag the release',
      'Publish the binaries',
    ]);
  });

  it('removes an item, and the whole list is rewritten', async () => {
    await canvas.openNote(title);
    await editor.removeItem(2);
    expect(await editor.itemTexts()).toEqual(['Write the changelog', 'Tag the release']);
    await editor.close();

    // The position *is* the identity, so a removal is a full replace, not a DELETE.
    expect(((await reread())?.items ?? []).map((item) => item.text)).toEqual([
      'Write the changelog',
      'Tag the release',
    ]);
  });

  it('carries the Markdown Rust rendered, not one the front end rebuilt', async () => {
    await canvas.openNote(title);
    await editor.toggleItem(1);
    await editor.close();

    // `- [x] ` is `notes::checklist::to_markdown`'s syntax, reaching the card as
    // `DisplayNote.copyText`; the front end holds no second copy of it.
    const copyText = await eventually(
      async () => (await reread())?.copyText,
      (text) => text?.includes('- [ ] Tag the release') === true,
      'the untick to come back in the rendered Markdown',
    );
    expect(copyText).toContain('- [x] Write the changelog');
    expect(copyText).toContain('- [ ] Tag the release');
  });

  /**
   * A todo list has no `content`: `C` copies its Markdown, as the card's own button does.
   * Opening the note and closing it leaves the canvas cursor on that card.
   */
  it('copies that same Markdown from the keyboard, and says which note', async function () {
    await canvas.openNote(title);
    await editor.close();

    await press('c');

    // The toast is the keyboard's only feedback: the card paints no tick for it.
    await banners.status().waitForExist({ timeout: 10_000 });
    expect(await banners.status().getText()).toContain(title);

    // An unreadable clipboard answers null at once, so only the readable case waits.
    const copied = await eventually(
      () => clipboardText(),
      (text) => text === null || text.includes('- ['),
      'the keyboard copy to reach the clipboard',
    );
    if (copied === null) {
      // No readable clipboard on this runner; see `clipboardText`.
      this.skip();
      return;
    }
    expect(copied).toContain('Tag the release');
  });

  it('puts that same Markdown on the clipboard', async function () {
    const card = await canvas.cardWithTitle(title);
    await card.$(testid('copy-button')).click();

    // An unreadable clipboard answers null at once, so only the readable case waits.
    const copied = await eventually(
      () => clipboardText(),
      (text) => text === null || text.includes('- [x] Write the changelog'),
      'the copy to reach the clipboard',
    );
    if (copied === null) {
      // No readable clipboard on this runner. Skipped rather than returned: a bare
      // `return` is a green test that asserted nothing. See `clipboardText`.
      this.skip();
      return;
    }
    expect(copied).toContain('- [x] Write the changelog');
  });

  /**
   * WCAG 2.2 AA (2.5.8) asks 24x24, and this row was 18: it is the control that was
   * reported as impossible to tick. Growing it is not free — the card is a fixed 150px and
   * `.card-items` is `overflow: hidden`, so the second half of this asserts the card still
   * shows what it claims rather than clipping a row the badge is still counting.
   */
  it('gives every control on a card a box a pointer can hit', async () => {
    await canvas.waitForCard(title);
    const sizes = await canvas.controlSizes();

    // The smallest side of each kind, named, so a failure says which control shrank rather
    // than only that one did.
    const smallest = Object.entries(sizes).map(([name, boxes]) => {
      expect(boxes.length).toBeGreaterThan(0);
      return [name, Math.min(...boxes.map((box) => Math.min(box.width, box.height)))] as const;
    });

    expect(smallest.filter(([, side]) => side < 24)).toEqual([]);
  });

  /**
   * `.card-items` is anchored to the bottom of 53px inside a 150px card: a row that does not fit
   * is drawn above its box and cut, while counting the rows still passes. So the boxes are read.
   */
  it('draws every row it shows inside the box that holds them', async () => {
    await canvas.waitForCard(title);
    const overflow = await canvas.rowOverflow(title);

    expect(overflow).not.toBeNull();
    expect(overflow?.length).toBeGreaterThan(0);
    expect(overflow).toEqual(overflow?.map(() => 0));
  });

  it('still shows the items it counts, rather than clipping one', async () => {
    const card = await canvas.cardWithTitle(title);
    const shown = (await card.$$(testid('note-card-item')).getElements()).length;
    const total = (await reread())?.items?.length ?? 0;

    // Two on a card, and the badge accounts for exactly the rest.
    expect(shown).toBe(Math.min(2, total));
    if (total > shown) {
      expect(await card.$(testid('note-card-more')).getText()).toContain(String(total - shown));
    }
  });

  it('shows progress on the card', async () => {
    const card = await canvas.cardWithTitle(title);
    const progress = await card.$('[data-testid="note-card-progress"]').getText();
    expect(progress).toContain('1');
    expect(progress).toContain('2');
  });

  /**
   * A card has two rows to say what a list is about, and what it is about is what is left.
   * The progress bar already says how much is done, so a ticked row costs a seat and
   * pays nothing back.
   */
  describe('a list with some of its items already done', () => {
    const partly = 'Partly done';
    const finished = 'Nothing left';
    const seeded: string[] = [];

    async function rows(of: string): Promise<string[]> {
      const card = await canvas.cardWithTitle(of);
      return card.$$(`${testid('note-card-item')} .item-text`).map((item) => item.getText());
    }

    before(async () => {
      const spaceId = await homeSpaceId();
      const first = await bridge.createNote(
        draft({
          spaceId,
          title: partly,
          kind: 'checklist',
          items: [
            { text: 'unpack the crate', done: true },
            { text: 'wire the relay', done: false },
            { text: 'seal the panel', done: false },
            { text: 'call it a day', done: false },
          ],
        }),
      );
      const second = await bridge.createNote(
        draft({
          spaceId,
          title: finished,
          kind: 'checklist',
          items: ['drain it', 'flush it', 'refill it'].map((text) => ({ text, done: true })),
        }),
      );
      seeded.push(first.id, second.id);
      await reloadCanvas();
      await canvas.waitForCard(partly);
    });

    // The canvas is shared with every spec file that runs after this one.
    after(async () => {
      await bridge.deleteNotes(seeded);
      await reloadCanvas();
    });

    it('spends its two rows on what is still to do', async () => {
      expect(await rows(partly)).toEqual(['wire the relay', 'seal the panel']);
    });

    it('counts the ones it left out, ticked or not', async () => {
      const card = await canvas.cardWithTitle(partly);

      expect(await card.$(testid('note-card-more')).getText()).toContain('2');
    });

    it('gives the row up as soon as it is ticked', async () => {
      const boxes = await (await canvas.cardWithTitle(partly)).$$(testid('note-card-item')).getElements();
      await boxes[0]!.click();

      const left = await eventually(
        () => rows(partly),
        (texts) => !texts.includes('wire the relay'),
        'the ticked row to leave the preview',
      );
      expect(left).toEqual(['seal the panel', 'call it a day']);
    });

    it('shows its last items rather than nothing when everything is done', async () => {
      expect(await rows(finished)).toEqual(['flush it', 'refill it']);
    });
  });

  /** A card shows two items of a list; a search keeps the matching one among them. */
  describe('found by an item the card does not show', () => {
    const long = 'Deep list';

    before(async () => {
      await bridge.createNote(
        draft({
          spaceId: await homeSpaceId(),
          title: long,
          kind: 'checklist',
          items: [
            { text: 'first step', done: false },
            { text: 'second step', done: false },
            { text: 'third step', done: false },
            { text: 'rotate the kubeconfig', done: false },
          ],
        }),
      );
      await reloadCanvas();
    });

    after(async () => {
      await canvas.clearSearch();
    });

    it('keeps the item that matched among the two it shows', async () => {
      await canvas.search('kubeconfig');
      const card = await canvas.cardWithTitle(long);
      const texts = await card.$$('[data-testid="note-card-item"]').map((item) => item.getText());

      expect(texts.join(' ')).toContain('rotate the kubeconfig');
      expect(texts.join(' ')).not.toContain('third step');
    });

    /**
     * A row is not at the place it holds in the note: ticking the second visible box
     * must not tick the second box of the list.
     */
    it('ticks the box it shows, not the one at the same place in the list', async () => {
      await canvas.search('kubeconfig');
      const card = await canvas.cardWithTitle(long);
      const boxes = await card.$$('[data-testid="note-card-item"]').getElements();
      // The second visible box, which is the last item of the list.
      await boxes[1]!.click();

      await browser.waitUntil(
        async () => {
          const view = await bridge.queryNotes(query({ search: long }));
          return view.sections[0]?.notes[0]?.items?.at(-1)?.done === true;
        },
        { timeout: 10_000, timeoutMsg: 'the matching item never came back ticked' },
      );

      const items = (await bridge.queryNotes(query({ search: long }))).sections[0]?.notes[0]?.items;
      expect(items?.map((item) => item.done)).toEqual([false, false, false, true]);
    });
  });
});
