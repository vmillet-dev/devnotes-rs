import { $, $$, browser } from '@wdio/globals';

import {
  blur,
  clickToAddRow,
  confirmTwice,
  eventually,
  press,
  readEach,
  setField,
  toggleAndWait,
  waitForCanvas,
  choiceLabel,
  pickChoice,
  setNativeValue,
  submitFormOf,
  testid,
} from '../support/app.js';

/**
 * Title, body and source commit on blur, so every setter here blurs before returning:
 * a spec asserting right after typing would be asserting on a draft nothing has saved.
 */
async function typeAndCommit(selector: string, text: string): Promise<void> {
  await setField(selector, text);
  await blur();
}

/** A Text note's body is the rich editor, loaded on demand; any other note's, the code field. */
async function bodyField(): Promise<'rich' | 'code'> {
  const field = await eventually(
    async () =>
      (await $(testid('editor-rich')).isExisting())
        ? 'rich'
        : (await $(testid('editor-body')).isExisting())
          ? 'code'
          : null,
    (found) => found !== null,
    'the body field to be drawn',
  );
  return field as 'rich' | 'code';
}

/** `setValue` reaches no contenteditable: the page inserts the text, which ProseMirror reads as typing. */
async function typeRich(text: string): Promise<void> {
  await browser.execute(
    (selector: string, value: string) => {
      (document.querySelector(selector) as HTMLElement).focus();
      document.execCommand('selectAll');
      document.execCommand('insertText', false, value);
    },
    testid('editor-rich'),
    text,
  );
  await eventually(
    () => $(testid('editor-rich')).getText(),
    (shown) => shown === text,
    'the rich editor to hold what was typed',
  );
  await blur();
}

async function rowAt(selector: string, index: number) {
  const rows = await $$(selector).getElements();
  const row = rows[index];
  if (!row) {
    throw new Error(`no row at ${index} for ${selector} — found ${rows.length}`);
  }
  return row;
}

export const editor = {
  /** Where the note is kept: its space, and its folder. */
  place: (kind: 'space' | 'folder', optionId: string | null) => pickChoice(kind, optionId),

  placementLabel: (kind: 'space' | 'folder') => choiceLabel(kind),

  isOpen: () => $(testid('editor-title')).isExisting(),

  setTitle: (text: string) => typeAndCommit(testid('editor-title'), text),
  async setBody(text: string): Promise<void> {
    if ((await bodyField()) === 'rich') {
      await typeRich(text);
    } else {
      await typeAndCommit(testid('editor-body'), text);
    }
  },
  setSource: (text: string) => typeAndCommit(testid('editor-source'), text),

  title: () => $(testid('editor-title')).getValue(),
  async body(): Promise<string> {
    return (await bodyField()) === 'rich'
      ? $(testid('editor-rich')).getText()
      : $(testid('editor-body')).getValue();
  },

  setLanguage: (language: string) => pickChoice('language', language),

  /** `yyyy-MM-dd`, which the editor converts to the end of the local day. */
  setDeadline: (isoDay: string) => setNativeValue(testid('editor-deadline'), isoDay),

  async addTag(tag: string): Promise<void> {
    await setField(testid('editor-tag-add'), tag);
    // The field commits by submitting its form, which Enter does natively and no
    // synthetic key can — see `submitFormOf`.
    await submitFormOf(testid('editor-tag-add'));

    // The field clearing only says the form was submitted; the list comes from the
    // note, after the write has crossed the bridge and come back. The comparison is loose
    // on purpose — `#` and case are `normalize_tags`'s to decide, and the scenario still
    // asserts the exact list Rust produced.
    const expected = tag.trim().replace(/^#/, '').toLowerCase();
    await browser.waitUntil(
      async () => (await editor.tags()).some((each) => each.toLowerCase() === expected),
      { timeout: 10_000, timeoutMsg: `the tag "${tag}" never reached the editor` },
    );
  },

  /** Waits for it to be gone: the list follows the write, not the click. */
  async removeTag(tag: string): Promise<void> {
    await $(`${testid('editor-tag-remove')}[data-tag="${tag}"]`).click();

    await browser.waitUntil(async () => !(await editor.tags()).includes(tag), {
      timeout: 10_000,
      timeoutMsg: `the tag "${tag}" is still on the note`,
    });
  },

  tags: (): Promise<string[]> => readEach(testid('editor-tag-remove'), '@data-tag'),

  togglePin: () => toggleAndWait(testid('editor-pin')),
  isPinned: async () => (await $(testid('editor-pin')).getAttribute('aria-pressed')) === 'true',

  footer: () => $(testid('editor-footer')).getText(),

  toggleFullscreen: () => toggleAndWait(testid('editor-fullscreen')),
  isFullscreen: async () => (await $(testid('editor-fullscreen')).getAttribute('aria-pressed')) === 'true',

  /** Measured: `aria-pressed` says the button was pressed, not that the panel grew. */
  panelSize: () => $('[role="dialog"]').getSize(),

  /** Only shown for a note carrying `{{fields}}`. */
  copyFilled: () => $(testid('editor-copy-filled')).click(),
  hasCopyFilled: () => $(testid('editor-copy-filled')).isExisting(),

  /** Escape, the backdrop and the button produce no `blur`: the component commits itself. */
  async close(): Promise<void> {
    await $(testid('editor-close')).click();
    await $(testid('editor-title')).waitForExist({ reverse: true, timeout: 10_000 });

    // Closing commits, and the dialog disappears without waiting for any of the three
    // writes. Settling the canvas is the observable end of that round trip.
    await waitForCanvas();
  },

  async deleteNote(): Promise<void> {
    await confirmTwice($(testid('editor-delete')));
    await $(testid('editor-title')).waitForExist({ reverse: true, timeout: 10_000 });
  },

  // Checklists
  items: () => $$(testid('checklist-row')),

  itemTexts: (): Promise<string[]> => readEach(testid('checklist-row'), 'value', testid('checklist-text')),

  async itemChecks(): Promise<boolean[]> {
    const states = await readEach(testid('checklist-row'), '@aria-checked', testid('checklist-check'));
    return states.map((state) => state === 'true');
  },

  async addItem(text: string): Promise<void> {
    const row = await clickToAddRow(testid('checklist-add'), testid('checklist-row'));
    await row.$(testid('checklist-text')).setValue(text);
    await blur();
  },

  async toggleItem(index: number): Promise<void> {
    await (await rowAt(testid('checklist-row'), index)).$(testid('checklist-check')).click();
    await blur();
  },

  async removeItem(index: number): Promise<void> {
    await (await rowAt(testid('checklist-row'), index)).$(testid('checklist-remove')).click();
    await blur();
  },

  /**
   * Asserted on, not dragged: synthesising a pointer capture through WebDriver is
   * unreliable and beside the point, `Alt+↑/↓` being its keyboard twin.
   */
  grip: (index: number) =>
    rowAt(testid('checklist-row'), index).then((row) => row.$(testid('checklist-grip'))),

  async moveItemUp(index: number): Promise<void> {
    const row = await rowAt(testid('checklist-row'), index);
    await row.$(testid('checklist-text')).click();
    await press('ArrowUp', ['Alt']);
    await blur();
  },

  // Fields
  fieldsPanelToggle: () => $(testid('placeholder-panel-toggle')),

  /** Trimmed: the count sits on its own line in the template, so `getText()` pads it. */
  fieldsPanelCount: async () => (await $(testid('placeholder-panel-count')).getText()).trim(),

  isFieldsPanelOpen: async () =>
    (await $(testid('placeholder-panel-toggle')).getAttribute('aria-expanded')) === 'true',

  /**
   * Scoped to the overlay: `placeholder-input` is the same hook in this panel and in
   * the fill form, and a bare `$()` returns whichever comes first in the DOM.
   */
  field: (name: string) => $(`app-note-editor-overlay ${testid('placeholder-input')}[data-field="${name}"]`),

  async toggleFieldsPanel(): Promise<void> {
    await $(testid('placeholder-panel-toggle')).click();
  },

  /** Open by default — a folded panel would hide the feature. */
  async openFieldsPanel(): Promise<void> {
    if (!(await editor.isFieldsPanelOpen())) {
      await editor.toggleFieldsPanel();
    }
  },

  // Attachments
  attachments: () => $$(testid('attachment-item')),
  attachmentEmpty: () => $(testid('attachment-empty')),

  attachmentNames: (): Promise<string[]> => readEach(testid('attachment-item'), '@data-file-name'),

  /**
   * Asserted on, never clicked — both open OS UI, and the picker blocks the whole
   * application until a human answers it. See `support/app.ts`.
   */
  attachmentAdd: () => $(testid('attachment-add')),
  attachmentOpen: (fileName: string) =>
    $(`${testid('attachment-item')}[data-file-name="${fileName}"]`).$(testid('attachment-open')),

  async removeAttachment(): Promise<void> {
    const remove = $(testid('attachment-remove'));
    await remove.click();
    await remove.click();
  },
};
