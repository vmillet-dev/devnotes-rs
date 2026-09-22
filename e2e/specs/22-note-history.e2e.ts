import { $, $$, expect } from '@wdio/globals';

import { canvas } from '../pageobjects/canvas.page.js';
import { editor } from '../pageobjects/editor.page.js';
import { eventually, testid } from '../support/app.js';

/**
 * The bodies kept beside a note.
 *
 * ⚠️ The trash protects a deletion and nothing protected an edit: you adjust a command
 * that worked, it stops working, and the version that worked is gone.
 */
describe('The history of a note', () => {
  const TITLE = 'Version gardée';

  const toggle = () => $(testid('revisions-toggle'));
  const rows = () => $$(testid('revision-row'));

  before(async () => {
    await canvas.open();
    // A snippet of this file's own: the seeded ones are what the earlier files assert on.
    await canvas.createSnippet();
    await editor.setTitle(TITLE);
    await editor.setBody('select 1');
    await editor.close();
  });

  /** ⚠️ A panel always there and always empty is a feature nobody uses. */
  it('shows nothing on a note nobody has edited', async () => {
    await canvas.openNote(TITLE);

    expect(await toggle().isExisting()).toBe(false);

    await editor.close();
  });

  it('appears once an edit has left a body behind', async () => {
    await canvas.openNote(TITLE);
    await editor.setBody('select 2');
    await editor.close();

    await canvas.openNote(TITLE);

    expect(
      await eventually(
        () => toggle().isExisting(),
        (there) => there,
        'the history to appear once a body has been replaced',
      ),
    ).toBe(true);
  });

  it('lists what was kept, and says how much each version held', async () => {
    await toggle().click();

    expect(await rows().length).toBe(1);
    // The stamp of a size, which no translation touches: "select 1" is eight characters.
    expect(await rows()[0].getText()).toContain('8');
  });

  /**
   * ⚠️ One click and no confirmation, deliberately: the body a restore replaces is kept
   * first, so it is as undoable as the edit that made it necessary. A guard in front of a
   * reversible gesture is how a safety net becomes a nuisance.
   */
  it('puts a body back on one click, and keeps the one it replaced', async () => {
    await $(testid('revision-restore')).click();

    expect(
      await eventually(
        () => editor.body(),
        (body) => body.includes('select 1'),
        'the kept body to go back onto the note',
      ),
    ).toContain('select 1');

    expect(
      await eventually(
        () => rows().length,
        (count) => count === 2,
        'the replaced body to be kept in its turn',
      ),
    ).toBe(2);

    await editor.close();
  });

  /** ⚠️ A checklist's items live in `note_items` — out of this first version, said so. */
  it('offers nothing on a todo list', async () => {
    await canvas.createChecklist();
    await editor.setTitle('Liste sans historique');
    await editor.close();
    await canvas.openNote('Liste sans historique');

    expect(await toggle().isExisting()).toBe(false);

    await editor.close();
  });
});
