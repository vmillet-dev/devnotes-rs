import { $, browser } from '@wdio/globals';

import { confirmTwice, setField, pickChoice, submitFormOf, testid } from '../support/app.js';

/** The breadcrumb, which replaces the switchers while a folder is open. */
export const crumb = {
  isShowing: () => $(testid('folder-breadcrumb')).isExisting(),

  name: () => $(testid('folder-breadcrumb-name')).getText(),

  back: () => $(testid('folder-breadcrumb-back')).click(),

  swatchClass: () =>
    browser.execute(
      (selector: string) => document.querySelector(selector)?.className ?? '',
      `${testid('folder-breadcrumb')} ~ * .crumb-swatch, ${testid('folder-breadcrumb')} .crumb-swatch`,
    ),

  /** ⚠️ Opens behind the ⋯, which is the only way to the folder's own actions from here. */
  async openMenu(): Promise<void> {
    if (!(await $(testid('folder-breadcrumb-panel')).isExisting())) {
      await $(testid('folder-breadcrumb-menu')).click();
      await $(testid('folder-breadcrumb-panel')).waitForExist({ timeout: 5_000 });
    }
  },

  async rename(into: string): Promise<void> {
    await crumb.openMenu();
    await setField(testid('folder-rename-input'), into);
    await $(testid('folder-rename-submit')).click();
  },

  /** ⚠️ No refuge to choose, unlike a space: the notes come out loose. */
  async remove(): Promise<void> {
    await crumb.openMenu();
    await confirmTwice($(testid('folder-delete')));
  },
};

/** Shown only while at least one card is ticked. */
export const selectionBar = {
  bar: () => $(testid('selection-bar')),
  count: () => $(testid('selection-count')).getText(),
  clear: () => $(testid('selection-clear')).click(),
  copy: () => $(testid('selection-copy')).click(),

  moveTo: (spaceId: string) => pickChoice('selection-move', spaceId),

  /** The same control both ways: the way out is an entry of the same menu. */
  fileInto: (folderId: string | null) => pickChoice('selection-file', folderId ?? '__unfile__'),

  async tag(tag: string): Promise<void> {
    const field = $(testid('selection-tag'));
    await field.click();
    await field.setValue(tag);
    await submitFormOf(testid('selection-tag'));
  },

  async delete(): Promise<void> {
    const button = $(testid('selection-delete'));
    await button.click();
    await button.click();
  },
};
