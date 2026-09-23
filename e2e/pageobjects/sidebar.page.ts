import { $, browser } from '@wdio/globals';

import { confirmTwice, readEach, setField, pickChoice, testid } from '../support/app.js';

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
