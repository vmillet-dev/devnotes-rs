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

  /** Measured, not asserted on a class: the marks share the title's row, and the body follows. */
  it('puts the title and the marks on one row, with the body right under it', async () => {
    const layout = await canvas.cardHeadLayout(title);

    expect(layout).not.toBeNull();
    // The row starts at the card's own edge, exactly like the snippet under it…
    expect(layout!.titleRowOffset).toBe(0);
    // …the marks sit at its right end and the title is what gives way to them…
    expect(layout!.marksRightAligned).toBe(true);
    // …that row is the first thing in the card, with no band above it…
    expect(layout!.titleTop).toBeLessThanOrEqual(16);
    // …and the body starts immediately after it.
    expect(layout!.snippetTop).toBeLessThanOrEqual(10);
    // …because the copy and ⋯ buttons hang under the card's bottom edge, clear of the marks.
    expect(layout!.actionsBelowBottom).toBe(true);
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
});
