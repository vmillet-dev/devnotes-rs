import { expect } from '@wdio/globals';

import { canvas } from '../pageobjects/canvas.page.js';
import { editor } from '../pageobjects/editor.page.js';
import { bottomGapOf, reopenSession, testid } from '../support/app.js';
import { bridge, query } from '../support/bridge.js';

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

  /**
   * ⚠️ Measured, not asserted on a class: the title used to run inline after the badge and
   * the marks, so it started in the middle of the card and what was left of it wrapped —
   * and every attempt at reserving a band for the buttons cost the title a line of its own.
   */
  it('gives the title the whole width, under the badge rather than beside it', async () => {
    const layout = await canvas.cardHeadLayout(title);

    expect(layout).not.toBeNull();
    // It starts at the card's own edge, exactly like the snippet under it…
    expect(layout!.offset).toBe(0);
    // …and it has as much room as the snippet, which nothing floats over.
    expect(layout!.titleWidth).toBe(layout!.snippetWidth);
    // …because the copy and ⋯ buttons hang over the card's top edge instead of taking a
    // band above it, which on a todo list with no marks held nothing else at all.
    expect(layout!.actionsAboveTop).toBe(true);
  });

  it('materialises the draft exactly once, not once per committed field', async () => {
    // ⚠️ Closing commits the title then the content with no change detection between
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

  /**
   * ⚠️ It used to be the last child of the scrolling canvas, so with a handful of notes
   * it floated in the middle of the window and with hundreds it was off the end of the
   * scroll. A legend belongs on the edge.
   */
  it('keeps the keyboard legend on the bottom edge whatever is above it', async () => {
    expect(await bottomGapOf(testid('canvas-keyboard-hint'))).toBeLessThanOrEqual(1);
  });

  it('reads the note back from the database on a fresh front end', async () => {
    // ⚠️ Not a process restart — see `reopenSession`. It proves the canvas renders what
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
});
