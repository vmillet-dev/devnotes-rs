import { $, $$, browser } from '@wdio/globals';

import { confirmTwice, readEach, submitFormOf, testid } from '../support/app.js';

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
  openRow: (index = 0) => $$(testid('palette-open'))[index]!,
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
