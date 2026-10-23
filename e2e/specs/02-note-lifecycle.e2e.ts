import { $, browser, expect } from '@wdio/globals';

import { canvas } from '../pageobjects/canvas.page.js';
import { editor } from '../pageobjects/editor.page.js';
import { bottomGapOf, press, reloadCanvas, reopenSession, testid } from '../support/app.js';
import { bridge, draft, homeSpaceId, query } from '../support/bridge.js';

/**
 * Creating a note writes nothing until it is worth saving, and the editor commits on the
 * way out — two front-end rules about a row in SQLite.
 */
describe('Creating a note, and finding it again', () => {
  const title = 'Rotate the staging certificate';
  const body = 'openssl req -new -key staging.key -out staging.csr';
  let corpusBefore = 0;

  before(canvas.open);

  it('opens a draft that is not yet a row', async () => {
    corpusBefore = (await bridge.queryNotes(query())).matched;
    await canvas.createSnippet();

    const during = await bridge.queryNotes(query());
    expect(during.matched).toBe(corpusBefore);
  });

  it('persists the note once it carries something worth keeping', async () => {
    await editor.setTitle(title);
    await editor.setBody(body);
    await editor.close();

    await canvas.waitForCard(title);
    const view = await bridge.queryNotes(query({ search: title }));
    expect(view.matched).toBe(1);
  });

  /** Measured, not asserted on a class: the marks share the title's row, and the body follows. */
  it('puts the title and its kind on one row, the marks in the footer, the body right under it', async () => {
    const layout = await canvas.cardHeadLayout(title);

    expect(layout).not.toBeNull();
    // The row starts at the card's own edge, exactly like the snippet under it…
    expect(layout!.titleRowOffset).toBe(0);
    // …the kind sits at its right end and the title is what gives way to it…
    expect(layout!.kindRightAligned).toBe(true);
    // …and no state mark sits on it: those live in the footer, where they cost the title nothing…
    expect(layout!.noMarkInRow).toBe(true);
    // …that row is the first thing in the card, with no band above it…
    expect(layout!.titleTop).toBeLessThanOrEqual(16);
    // …and the body starts immediately after it.
    expect(layout!.snippetTop).toBeLessThanOrEqual(10);
    // …because the copy and ⋯ buttons hang under the card's bottom edge, clear of the marks.
    expect(layout!.actionsBelowBottom).toBe(true);
  });

  /** The date is what a footer is for: the tags and the folder chip give way before it does. */
  it('keeps the deadline whole in a footer full of a folder and tags', async () => {
    const crowded = 'Renew the wildcard certificate';
    const spaceId = await homeSpaceId();
    const folder = await bridge.createFolder({ spaceId, name: 'Infrastructure et réseau' });
    const { id } = await bridge.createNote(
      draft({
        spaceId,
        title: crowded,
        content: 'certbot renew',
        tags: ['certificates', 'kubernetes', 'production'],
        lifecycle: { kind: 'expires', at: new Date(Date.now() + 2 * 86_400_000).toISOString() },
      }),
    );
    await bridge.fileNotes([id], folder.id);
    await reloadCanvas();
    await canvas.waitForCard(crowded);

    const when = await browser.execute((wanted: string) => {
      const shell = [...document.querySelectorAll('[data-testid="note-card"]')].find(
        (each) =>
          (each.querySelector('[data-testid="note-card-title"]')?.textContent ?? '').trim() === wanted,
      );
      const label = shell?.querySelector('.card-when');
      return label
        ? { whole: label.scrollWidth <= label.clientWidth, text: label.textContent?.trim() }
        : null;
    }, crowded);
    expect(when?.whole).toBe(true);

    await bridge.deleteNotes([id]);
    await bridge.purgeNotes([id]);
    await bridge.deleteFolder(folder.id);
    await reloadCanvas();
  });

  it('materialises the draft exactly once, not once per committed field', async () => {
    // Closing commits the title then the content with no change detection between
    // them: the second call still carries `DRAFT_ID` while the row already exists.
    expect((await bridge.queryNotes(query())).matched).toBe(corpusBefore + 1);
  });

  it('opens the same draft from the ghost card at the end of the week section', async () => {
    // The `week` section is always emitted precisely so this card has a home.
    await canvas.createFromGhost();
    expect(await editor.isOpen()).toBe(true);
    await editor.close();

    // Still a draft nothing kept: closing an untouched one writes no row.
    expect((await bridge.queryNotes(query())).matched).toBe(corpusBefore + 1);
  });

  it('offers the same thing from the kind menu as from the split button', async () => {
    await canvas.createSnippetFromMenu();
    expect(await editor.isOpen()).toBe(true);
    await editor.close();
    expect((await bridge.queryNotes(query())).matched).toBe(corpusBefore + 1);
  });

  /** The keys the kind menu names beside each kind: one draft each, and no row left behind. */
  it('opens each kind of note from its own key', async () => {
    for (const [key, modifiers, body] of [
      ['n', ['Control'], 'editor-body'],
      ['N', ['Control', 'Shift'], 'editor-rich'],
      ['L', ['Control', 'Shift'], 'checklist-add'],
    ] as const) {
      await press(key, [...modifiers]);
      await $(testid(body)).waitForExist({ timeout: 10_000 });
      await editor.close();
    }

    expect((await bridge.queryNotes(query())).matched).toBe(corpusBefore + 1);
  });

  /** Whatever an earlier file did with the menu: opening a Note is what retires the badge. */
  it('stops calling the Note new once one has been opened', async () => {
    await canvas.createRichNote();
    await editor.close();

    await $(testid('new-note-kind')).click();
    await $(testid('new-note-note')).waitForExist({ timeout: 10_000 });
    expect(await $(testid('new-note-badge')).isExisting()).toBe(false);
    await press('Escape');
    await $(testid('new-note-note')).waitForExist({ reverse: true, timeout: 10_000 });
  });

  /** A legend belongs on the edge, whatever the canvas holds above it. */
  it('keeps the keyboard legend on the bottom edge whatever is above it', async () => {
    expect(await bottomGapOf(testid('canvas-keyboard-hint'))).toBeLessThanOrEqual(1);
  });

  it('reads the note back from the database on a fresh front end', async () => {
    // Not a process restart — see `reopenSession`. It proves the canvas renders what
    // the commands answer, not something a signal was still holding.
    await reopenSession();

    await canvas.waitForCard(title);
    const view = await bridge.queryNotes(query({ search: title }));
    expect(view.matched).toBe(1);

    const section = view.sections[0];
    const note = section?.notes[0];
    expect(note?.title).toBe(title);
    expect(note?.content).toBe(body);
  });

  /**
   * NOT-02: a title typed into a new note, then Escape with the caret still in it, and no
   * card was made. `editor.setTitle` blurs the field, and the blur committed what Escape lost.
   */
  it('keeps a new note titled and closed with Escape, the caret still in its title', async () => {
    const escaped = 'Closed with Escape';
    await canvas.createSnippet();
    await $(testid('editor-title')).click();
    await browser.keys([...escaped]);
    expect(await browser.execute(() => document.activeElement?.getAttribute('data-testid'))).toBe(
      'editor-title',
    );
    await browser.keys(['Escape']);
    await $(testid('editor-title')).waitForExist({ reverse: true, timeout: 10_000 });

    await canvas.waitForCard(escaped);
    await reloadCanvas();
    await canvas.waitForCard(escaped);
    const kept = (await bridge.queryNotes(query({ search: escaped }))).sections[0]?.notes[0];
    expect(kept?.title).toBe(escaped);

    await bridge.deleteNotes([kept!.id]);
    await bridge.purgeNotes([kept!.id]);
    await reloadCanvas();
  });
});
