import { expect } from '@wdio/globals';

import { canvas } from '../pageobjects/canvas.page.js';
import { editor } from '../pageobjects/editor.page.js';
import { fieldsForm } from '../pageobjects/overlays.page.js';
import { settings, variables } from '../pageobjects/titlebar.page.js';
import { clipboardText, eventually, press, reloadCanvas } from '../support/app.js';
import { bridge, draft, homeSpaceId, query } from '../support/bridge.js';

/**
 * A `{{field}}` is decided in `notes::placeholder` and nowhere else, and a global variable
 * only ever proposes a value: it reaches the card as `defaultValue`, and copying it into
 * `value` would freeze it.
 */
describe('{{fields}} in a snippet', () => {
  const title = 'Connect to the database';
  const body = 'psql -h {{host}} -p {{port=5432}} -U {{user}}';

  before(async () => {
    await canvas.open();
    const spaceId = await homeSpaceId();
    await bridge.createNote(draft({ spaceId, title, content: body, language: 'sh' }));
    await reloadCanvas();
    await canvas.waitForCard(title);
  });

  async function reread() {
    const view = await bridge.queryNotes(query({ search: title }));
    return view.sections[0]?.notes[0];
  }

  it('finds the three fields, and only those', async () => {
    const names = (await reread())?.placeholders.map((field) => field.name);
    expect(names).toEqual(['host', 'port', 'user']);
  });

  it('shows the ⚡ affordance on the card instead of a plain copy', async () => {
    const card = await canvas.cardWithTitle(title);
    expect(await card.$('[data-testid="note-card-fields"]').isExisting()).toBe(true);
    expect(await card.$('[data-testid="note-card-fill"]').isExisting()).toBe(true);
  });

  /**
   * The keyboard went straight past the form and pasted the tokens. Opening the note
   * and closing it is what leaves the canvas cursor on that card.
   */
  it('asks for the fields when the copy comes from the keyboard too', async () => {
    await canvas.openNote(title);
    await editor.close();

    await press('c');

    await fieldsForm.form().waitForExist({ timeout: 10_000 });
    await fieldsForm.cancel();
  });

  it('carries the default written in the text, as a suggestion', async () => {
    const port = (await reread())?.placeholders.find((field) => field.name === 'port');
    expect(port?.defaultValue).toBe('5432');
  });

  it('shows the fields panel open, counting what is filled', async () => {
    await canvas.openNote(title);

    // Open by default: folded away it hides the feature.
    expect(await editor.isFieldsPanelOpen()).toBe(true);

    // Nothing typed yet, and the default written in the text does not count as filled.
    expect(await editor.fieldsPanelCount()).toBe('0/3');
    await editor.close();
  });

  it('folds the panel away, and remembers it was folded', async () => {
    await canvas.openNote(title);
    await editor.toggleFieldsPanel();
    expect(await editor.isFieldsPanelOpen()).toBe(false);
    await editor.close();

    // A preference, not a per-note state: reopening keeps it folded.
    await canvas.openNote(title);
    expect(await editor.isFieldsPanelOpen()).toBe(false);

    await editor.openFieldsPanel();
    expect(await editor.isFieldsPanelOpen()).toBe(true);
    await editor.close();
  });

  it('swaps the plain copy for one that fills the fields first', async function () {
    await canvas.openNote(title);
    // A note with fields copies filled; "copy as is" stays within reach in the panel.
    expect(await editor.hasCopyFilled()).toBe(true);

    await editor.copyFilled();
    // An unreadable clipboard answers null at once, so only the readable case waits.
    const filled = await eventually(
      () => clipboardText(),
      (text) => text === null || text.includes('-p 5432'),
      'the filled copy to reach the clipboard',
    );
    await editor.close();

    if (filled === null) {
      this.skip();
      return;
    }
    // `fill_placeholders` reads the database, so the default written in the text stands
    // in for a field nothing has typed into yet.
    expect(filled).toContain('-p 5432');
    expect(filled).not.toContain('{{port');
  });

  it('copies the snippet as it is, tokens included, from the panel', async function () {
    const card = await canvas.cardWithTitle(title);
    await card.$('[data-testid="note-card-fill"]').click();
    await fieldsForm.form().waitForExist({ timeout: 10_000 });

    await fieldsForm.copyRaw();
    // Copies and dismisses — there is no form left to cancel.
    await fieldsForm.form().waitForExist({ reverse: true, timeout: 10_000 });

    const raw = await clipboardText();
    if (raw === null) {
      this.skip();
      return;
    }
    // "As is" means the text, not the form's answer — the tokens survive.
    expect(raw).toContain('{{host}}');
    expect(raw).toContain('{{port=5432}}');
  });

  it('keeps what was typed into the form', async () => {
    const card = await canvas.cardWithTitle(title);
    await card.$('[data-testid="note-card-fill"]').click();
    await fieldsForm.form().waitForExist({ timeout: 10_000 });

    await fieldsForm.field('host').setValue('db.internal');
    await fieldsForm.field('user').setValue('reader');
    await fieldsForm.submit();
    await fieldsForm.form().waitForExist({ reverse: true, timeout: 10_000 });

    const fields = await eventually(
      async () => (await reread())?.placeholders ?? [],
      (stored) => stored.find((field) => field.name === 'host')?.value === 'db.internal',
      'the typed values to be stored',
    );
    expect(fields.find((field) => field.name === 'host')?.value).toBe('db.internal');
    expect(fields.find((field) => field.name === 'user')?.value).toBe('reader');
  });

  it('counts those two in the editor panel', async () => {
    await canvas.openNote(title);
    await editor.openFieldsPanel();
    expect(await editor.fieldsPanelCount()).toBe('2/3');
    expect(await editor.field('host').getValue()).toBe('db.internal');
    await editor.close();
  });

  it('leaves the form without writing anything when it is cancelled', async () => {
    const card = await canvas.cardWithTitle(title);
    await card.$('[data-testid="note-card-fill"]').click();
    await fieldsForm.form().waitForExist({ timeout: 10_000 });

    await fieldsForm.field('host').setValue('db.discarded');
    await fieldsForm.cancel();
    await fieldsForm.form().waitForExist({ reverse: true, timeout: 10_000 });

    const fields = (await reread())?.placeholders ?? [];
    expect(fields.find((field) => field.name === 'host')?.value).toBe('db.internal');
  });

  it('does not refresh updated_at, which the canvas sorts on', async () => {
    // Filling a field is not aimed at the note: `set_placeholder_values` has a command of
    // its own precisely so it does not take the patch path.
    const before = (await reread())?.updatedAt;
    const card = await canvas.cardWithTitle(title);
    await card.$('[data-testid="note-card-fill"]').click();
    await fieldsForm.form().waitForExist({ timeout: 10_000 });
    await fieldsForm.field('host').setValue('db.other');
    await fieldsForm.submit();
    // The form closing is its own condition, and the next scenario opens a panel over
    // this one: waiting on the stored value alone let the two dialogs overlap.
    await fieldsForm.form().waitForExist({ reverse: true, timeout: 10_000 });

    // The write is waited on through the value, then the column it must *not* have moved.
    const after = await eventually(
      () => reread(),
      (note) => note?.placeholders.find((field) => field.name === 'host')?.value === 'db.other',
      'the new value to be stored',
    );
    expect(after?.updatedAt).toBe(before);
  });

  it('lets a global variable propose a value without freezing it', async () => {
    await variables.open();
    await variables.add('port', '6543');
    await settings.close();
    await reloadCanvas();

    const port = (await reread())?.placeholders.find((field) => field.name === 'port');
    // The proposal arrives as the default, not as the stored value.
    expect(port?.defaultValue).toBe('6543');
    expect(port?.value).toBe('');
    expect(await bridge.listGlobalPlaceholders()).toEqual({ port: '6543' });
  });

  it('gives the snippet its own default back when the variable is removed', async () => {
    await variables.open();
    await variables.remove('port');
    await settings.close();
    await reloadCanvas();

    const port = (await reread())?.placeholders.find((field) => field.name === 'port');
    // Back to the default written in the text: the note never stored the proposal.
    expect(port?.defaultValue).toBe('5432');
    expect(await bridge.listGlobalPlaceholders()).toEqual({});
  });
});
