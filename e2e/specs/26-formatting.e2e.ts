import { $, $$, expect } from '@wdio/globals';

import { canvas } from '../pageobjects/canvas.page.js';
import { editor } from '../pageobjects/editor.page.js';
import { spaces } from '../pageobjects/sidebar.page.js';
import { checkedSegment, eventually, pickSegment, press, reloadCanvas, testid } from '../support/app.js';
import { bridge, draft } from '../support/bridge.js';

/**
 * Prettier runs in a worker the assembled application loads under its CSP, and writes through
 * the field's own editing: the two things no unit spec reaches.
 */
describe('Formatting a snippet with Prettier', () => {
  const TITLE = 'Invoices client';
  const MESSY =
    "import {inject} from '@angular/core'\nconst host = {{db_host}}\nexport class InvoicesApi{\nprivate readonly http=inject(HttpClient)\nlist(id:string){return this.http.get(`/api/customers/${id}/invoices`,{params:{page:1,limit:50}})}\n}";
  // The library style: single quotes, semicolons, 100 columns — and the field left as written.
  const TIDY =
    "import { inject } from '@angular/core';\nconst host = {{db_host}};\nexport class InvoicesApi {\n  private readonly http = inject(HttpClient);\n  list(id: string) {\n    return this.http.get(`/api/customers/${id}/invoices`, { params: { page: 1, limit: 50 } });\n  }\n}";
  const BROKEN = 'Broken on purpose';
  const QUERY = 'Unformattable query';
  const ids: string[] = [];
  let spaceId = '';

  const formatButton = () => $(testid('editor-format'));
  const notice = () => $(testid('editor-format-notice'));

  function bodyBecomes(expected: string, what: string): Promise<string> {
    return eventually(
      () => editor.body(),
      (body) => body === expected,
      what,
    );
  }

  before(async () => {
    await canvas.open();
    // A space of its own: the first launch's went with the library 24-forgotten-passphrase set aside.
    spaceId = (await bridge.createSpace({ name: 'Prettier' })).id;
    for (const [title, content, language] of [
      [TITLE, MESSY, 'ts'],
      [BROKEN, 'const a = {\n  b: 1,,\n}', 'ts'],
      [QUERY, 'select  *  from notes', 'sql'],
    ] as const) {
      ids.push((await bridge.createNote(draft({ spaceId, title, content, language }))).id);
    }
    await reloadCanvas();
    await spaces.open();
    await spaces.option(spaceId).click();
    await canvas.waitForCard(TITLE);
  });

  after(async () => {
    await bridge.deleteNotes(ids);
    await bridge.purgeNotes(ids);
    await reloadCanvas();
  });

  it('formats from the button, marks the lines it changed and says how many', async () => {
    await canvas.openNote(TITLE);
    await formatButton().click();

    expect(await bodyBecomes(TIDY, 'the formatted text in the field')).toBe(TIDY);
    await notice().waitForExist({ timeout: 5_000 });
    expect(await notice().getAttribute('data-kind')).toBe('formatted');
    expect((await $$('.editor-stack .code-line.marked').getElements()).length).toBeGreaterThan(0);
  });

  /** The field's own undo: a format is one step of it, like a keystroke. */
  it('takes the format back from its notice', async () => {
    await $(testid('editor-format-undo')).click();

    expect(await bodyBecomes(MESSY, 'the text as it was typed')).toBe(MESSY);
    expect(await notice().isExisting()).toBe(false);
  });

  it('formats on Shift+Alt+F from anywhere in the editor, and saves the result', async () => {
    // From the title: the shortcut belongs to the editor, not to the code field alone.
    await $(testid('editor-title')).click();
    await press('F', ['Shift', 'Alt']);
    await bodyBecomes(TIDY, 'the formatted text in the field');
    await editor.close();

    const stored = await eventually(
      async () => (await bridge.getNote(ids[0]!)).content,
      (content) => content === TIDY,
      'the formatted text to reach the database',
    );
    expect(stored).toBe(TIDY);
  });

  it('says where a syntax error is, and changes nothing', async () => {
    await canvas.openNote(BROKEN);
    await formatButton().click();

    await notice().waitForExist({ timeout: 10_000 });
    expect(await notice().getAttribute('data-kind')).toBe('syntax');
    expect(await editor.body()).toBe('const a = {\n  b: 1,,\n}');
    await editor.close();
  });

  it('leaves the button disabled on a language Prettier does not format', async () => {
    await canvas.openNote(QUERY);

    expect(await formatButton().getAttribute('aria-disabled')).toBe('true');
    expect(await $(testid('editor-prettier')).isExisting()).toBe(false);
    await editor.close();
  });

  describe('the library settings', () => {
    const SAVED = 'Formatted on save';
    // Double quotes and four spaces, as the panel is about to ask.
    const RESTYLED =
      'import { inject } from "@angular/core";\nconst host = {{db_host}};\nexport class InvoicesApi {\n    private readonly http = inject(HttpClient);\n    list(id: string) {\n        return this.http.get(`/api/customers/${id}/invoices`, { params: { page: 1, limit: 50 } });\n    }\n}';

    async function openPanel(): Promise<void> {
      await $(testid('editor-format-settings')).click();
      await $(testid('format-panel')).waitForExist({ timeout: 5_000 });
    }

    async function closePanel(): Promise<void> {
      await press('Escape');
      await $(testid('format-panel')).waitForExist({ reverse: true, timeout: 5_000 });
    }

    before(async () => {
      ids.push((await bridge.createNote(draft({ spaceId, title: SAVED, content: 'a', language: 'ts' }))).id);
      await reloadCanvas();
      await canvas.waitForCard(SAVED);
    });

    /** Back to the defaults: the library's preferences outlive this file. */
    after(async () => {
      await canvas.openNote(TITLE);
      await openPanel();
      await pickSegment('prettier-quotes', 'single');
      await pickSegment('prettier-indentation', 'editor');
      if (await $(testid('prettier-formatOnSave')).isSelected()) {
        await $(testid('prettier-formatOnSave')).click();
      }
      await closePanel();
      await editor.close();
    });

    it('formats in the style chosen in its panel', async () => {
      await canvas.openNote(TITLE);
      await openPanel();
      await pickSegment('prettier-quotes', 'double');
      await pickSegment('prettier-indentation', 'four');
      await closePanel();

      await formatButton().click();

      expect(await bodyBecomes(RESTYLED, 'the text in the chosen style')).toBe(RESTYLED);
      await editor.close();
    });

    it('keeps the choices in the library', async () => {
      await reloadCanvas();
      await canvas.openNote(TITLE);
      await openPanel();

      expect(await checkedSegment('prettier-quotes')).toBe('double');
      expect(await checkedSegment('prettier-indentation')).toBe('four');
      await closePanel();
      await editor.close();
    });

    it('formats what the field commits once asked to', async () => {
      await canvas.openNote(SAVED);
      await openPanel();
      await $(testid('prettier-formatOnSave')).click();
      await closePanel();

      await editor.setBody('const a={b:1}');

      const stored = await eventually(
        async () => (await bridge.getNote(ids.at(-1)!)).content,
        (content) => content === 'const a = { b: 1 };',
        'the formatted text to be what is saved',
      );
      expect(stored).toBe('const a = { b: 1 };');
      await editor.close();
    });
  });
});
