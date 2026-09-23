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

  /** ⚠️ Reopening the same note keeps the panel as it was left: open it only if shut. */
  async function showHistory(): Promise<void> {
    if ((await toggle().getAttribute('aria-expanded')) !== 'true') {
      await toggle().click();
    }
  }

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
    expect(await rows()[0]!.getText()).toContain('8');
  });

  /** A → B → C: "select 1", then "select 2", now "select 3" on screen. */
  it('keeps the newest version first', async () => {
    await editor.setBody('select 3');
    await editor.close();
    await canvas.openNote(TITLE);
    await showHistory();

    expect(
      await eventually(
        () => rows().length,
        (count) => count === 2,
        'the second edit to leave its body behind',
      ),
    ).toBe(2);
  });

  /**
   * ⚠️ Going back is irreversible, so a row opens a preview and never restores on its own
   * — and the preview shows what would change, which a date and a size never did (#325).
   */
  it('opens a preview on a click, showing what going back would change', async () => {
    await $$(testid('revision-open'))[0]!.click();

    const diff = await eventually(
      () => $(testid('revision-diff')).getText(),
      (text) => text.includes('select 2'),
      'the preview to compare the version with the current text',
    );
    expect(diff).toContain('select 3');
    expect(await $(testid('diff-restored')).getText()).toContain('select 2');
    expect(await $(testid('diff-dropped')).getText()).toContain('select 3');
    expect(await editor.body()).toContain('select 3');
  });

  /** The reporter's own words: A → B → C, go back to B, and C no longer exists. */
  it('goes back from the preview, and the version and everything newer leave the list', async () => {
    await $(testid('revision-restore')).click();

    expect(
      await eventually(
        () => editor.body(),
        (body) => body.includes('select 2'),
        'the kept body to go back onto the note',
      ),
    ).toContain('select 2');

    expect(
      await eventually(
        () => rows().length,
        (count) => count === 1,
        'the history to keep only what came before',
      ),
    ).toBe(1);
    expect(await rows()[0]!.getText()).toContain('8');

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
