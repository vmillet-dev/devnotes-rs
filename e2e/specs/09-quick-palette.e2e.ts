import { $, browser, expect } from '@wdio/globals';

import { canvas } from '../pageobjects/canvas.page.js';
import { editor } from '../pageobjects/editor.page.js';
import { fieldsForm, palette } from '../pageobjects/overlays.page.js';
import { aboutMenu } from '../pageobjects/titlebar.page.js';
import {
  emitGlobalAction,
  eventually,
  isInFront,
  press,
  readEach,
  reloadCanvas,
  testid,
} from '../support/app.js';
import { bridge, draft, homeSpaceId, query } from '../support/bridge.js';

/**
 * The OS-level `Ctrl+Alt+P` is out of scope: WebDriver types into the WebView, not into
 * the machine. What is exercised is everything downstream of it — the same action event
 * the accelerator and the tray both send.
 */
describe('The quick-paste palette', () => {
  async function setShortcuts(bindings: Record<string, string>) {
    return (await browser.executeAsync((payload: Record<string, string>, done: (value: unknown) => void) => {
      const tauri = (window as unknown as Record<string, any>)['__TAURI__'];
      tauri.core
        .invoke('set_global_shortcuts', { bindings: payload })
        .then((value: unknown) => done({ ok: value }))
        .catch((error: unknown) => done({ err: String(error) }));
    }, bindings)) as { ok?: string[]; err?: string };
  }

  before(async () => {
    await canvas.open();
    const spaceId = await homeSpaceId();
    await bridge.createNote(
      draft({ spaceId, title: 'Reset the dev database', content: 'cargo run -- reset', language: 'sh' }),
    );
    await bridge.createNote(draft({ spaceId, title: 'Unrelated note', content: 'nothing to see' }));
    await bridge.createNote(
      draft({ spaceId, title: 'Templated connection', content: 'psql -h {{host}}', language: 'sh' }),
    );
    await reloadCanvas();
  });

  it('opens on the action the native side sends', async () => {
    await emitGlobalAction('palette');
    await palette.input().waitForExist({ timeout: 10_000 });
  });

  it('narrows to what was typed', async () => {
    await palette.type('reset');
    const titles = await palette.titles();
    expect(titles.join(' ')).toContain('Reset the dev database');
    expect(titles.join(' ')).not.toContain('Unrelated note');
  });

  it('offers to create a note from a query that matches nothing', async () => {
    await palette.type('a query matching nothing at all');
    expect(await palette.createRow().isExisting()).toBe(true);
    // The offer replaces the list rather than sitting under an empty one.
    expect(await palette.options().length).toBe(0);
  });

  /** What was typed while looking for a note is its name: the note opens titled, already saved. */
  it('creates a note named after the query from its create row', async () => {
    const wanted = 'Rotation des certificats wildcard';
    await palette.type(wanted);
    await palette.createRow().$('button').click();

    await editor.waitOpen();
    expect(await editor.title()).toBe(wanted);
    await editor.close();

    const created = (await bridge.queryNotes(query({ search: wanted }))).sections[0]?.notes[0];
    expect(created?.title).toBe(wanted);
    await bridge.deleteNotes([created!.id]);
    await bridge.purgeNotes([created!.id]);
    await emitGlobalAction('palette');
    await palette.input().waitForExist({ timeout: 10_000 });
  });

  it('says the corpus is empty rather than showing a bare list', async () => {
    await palette.type('');
    // Every note is a candidate with no query, so the empty row must not appear.
    expect(await palette.empty().isExisting()).toBe(false);
    expect(await palette.options().length).toBeGreaterThan(0);
  });

  /** Enter opens, as a click on the row does: the first thing anyone does with a result. */
  it('opens the highlighted note in the editor on Enter', async () => {
    await palette.type('reset');
    await press('Enter');

    expect(await editor.isOpen()).toBe(true);
    expect(await editor.title()).toBe('Reset the dev database');
    await editor.close();
  });

  /** PAL-02: « psql » opened the note edited last that held it in its body, « Connexion psql » fourth. */
  it('puts a title holding the words before a note edited later that holds them in its body', async () => {
    const spaceId = await homeSpaceId();
    const titled = await bridge.createNote(
      // A word no sample holds: in English the samples' « psql connection » starts with psql.
      draft({ spaceId, title: 'Connexion zqsql', content: 'zqsql -U ada billing', language: 'sh' }),
    );
    const inBody = await bridge.createNote(
      draft({ spaceId, title: 'zq-champs', content: 'then run zqsql again', language: 'sh' }),
    );
    try {
      if (!(await palette.input().isExisting())) {
        await emitGlobalAction('palette');
        await palette.input().waitForExist({ timeout: 10_000 });
      }
      await palette.type('zqsql');
      const found = await eventually(
        () => readEach(`${testid('palette-option')}[data-kind="note"]`, '@data-note-id'),
        (ids) => ids.includes(inBody.id),
        'the notes holding zqsql',
      );
      expect(found[0]).toBe(titled.id);
    } finally {
      await bridge.deleteNotes([titled.id, inBody.id]);
      await bridge.purgeNotes([titled.id, inBody.id]);
    }
  });

  /** The palette ignores the canvas filters, so its results often name a note no view holds. */
  it('opens a result the canvas filters are hiding', async () => {
    await press('Escape');
    await canvas.search('Unrelated note');
    await canvas.waitForNoCard('Reset the dev database');

    await emitGlobalAction('palette');
    await palette.input().waitForExist({ timeout: 10_000 });
    await palette.type('reset');
    await press('Enter');

    expect(await editor.isOpen()).toBe(true);
    expect(await editor.title()).toBe('Reset the dev database');

    await editor.close();
    // The filter is the user's and the palette does not touch it.
    expect(await canvas.searchQuery()).toBe('Unrelated note');
    await canvas.clearSearch();
  });

  /**
   * The toast itself is drawn by the operating system after the window has gone. What the suite
   * can prove is the half that fails silently: a capability missing from
   * `capabilities/default.json` makes the plugin refuse at runtime, and nothing says so.
   */
  it('is allowed to ask the desktop whether it may speak', async () => {
    const answer = (await browser.executeAsync((done: (value: unknown) => void) => {
      const tauri = (window as unknown as Record<string, any>)['__TAURI__'];
      tauri.core
        .invoke('plugin:notification|is_permission_granted')
        .then((value: unknown) => done({ ok: value }))
        .catch((error: unknown) => done({ err: String(error) }));
    })) as { ok?: unknown; err?: string };

    expect(answer.err).toBeUndefined();
  });

  /** Tab walks the results like an arrow: one current row, drawn, and the field keeps the keyboard. */
  it('walks the results with Tab, without letting the field lose the keyboard', async () => {
    // Nothing above it leaves the palette open, so this one opens its own.
    await emitGlobalAction('palette');
    await palette.input().waitForExist({ timeout: 10_000 });
    await palette.type('');
    expect(await palette.options().length).toBeGreaterThan(1);

    const first = await palette.current();

    await press('Tab');
    const second = await palette.current();
    await press('Tab', ['Shift']);

    expect(second).not.toBe(first);
    expect(await palette.current()).toBe(first);
    expect(await palette.focused()).toBe('palette-input');
  });

  /**
   * Asked of a snippet with fields on purpose: the copy path proper ends in
   * `window.hide()`, and this run shares one window with every file after it. The form is
   * where `Ctrl+C` lands for this note, which proves the binding reaches the copy without
   * putting the window away.
   */
  it('asks for the fields on Ctrl+C, which is the copy path', async () => {
    await emitGlobalAction('palette');
    await palette.input().waitForExist({ timeout: 10_000 });
    await palette.type('Templated');

    await press('c', ['Control']);

    await fieldsForm.form().waitForExist({ timeout: 10_000 });
    await fieldsForm.cancel();
    expect(await editor.isOpen()).toBe(false);
  });

  /** A click opens, with the window still on screen: nothing is copied behind the user's back. */
  it('opens the note that was clicked, with the window still on screen', async () => {
    await emitGlobalAction('palette');
    await palette.input().waitForExist({ timeout: 10_000 });
    await palette.type('reset');

    await palette.openRow().click();

    expect(await editor.isOpen()).toBe(true);
    expect(await editor.title()).toBe('Reset the dev database');
    await editor.close();
  });

  /**
   * The one way to a note while a help panel is up: the panel covers the page, and a global
   * shortcut comes from outside. The help is drawn over the editor, so the editor puts it away.
   */
  it('opens a note in front of a help panel that was left open', async () => {
    await aboutMenu.openGettingStarted();

    await emitGlobalAction('palette');
    await palette.input().waitForExist({ timeout: 10_000 });
    await palette.type('reset');
    await press('Enter');

    await editor.title();
    expect(await isInFront(testid('editor-title'))).toBe(true);
    expect(await aboutMenu.gettingStarted().isExisting()).toBe(false);

    await editor.close();
  });

  it('closes on Escape', async () => {
    await emitGlobalAction('palette');
    await palette.input().waitForExist({ timeout: 10_000 });
    await press('Escape');
    await palette.input().waitForExist({ reverse: true, timeout: 10_000 });
  });

  describe('beyond the notes', () => {
    let requestId = '';
    const row = (attribute: string, value: string) =>
      $(`${testid('palette-option')}[${attribute}="${value}"]`);

    before(async () => {
      const collection = await bridge.createHttpCollection('Palette API');
      requestId = (
        await bridge.createHttpRequest({
          collectionId: collection.id,
          folderId: null,
          name: 'Rotate the signing keys',
          kind: 'http',
          method: 'POST',
          document: { url: '{{baseUrl}}/keys/rotate', description: '' },
        })
      ).id;
    });

    it('finds a request by its URL and a tool by a keyword, each with its own mark', async () => {
      await emitGlobalAction('palette');
      await palette.input().waitForExist({ timeout: 10_000 });

      await palette.type('keys/rot');
      await row('data-request-id', requestId).waitForExist({ timeout: 10_000 });
      expect(await row('data-request-id', requestId).$('.method').getAttribute('data-method')).toBe('POST');

      await palette.type('crontab');
      await row('data-tool', 'cron').waitForExist({ timeout: 10_000 });

      // PAL-07: the tool named « JSON » first, never cut by the three a kind keeps.
      await palette.type('json');
      await eventually(
        () => readEach(`${testid('palette-option')}[data-kind="tool"]`, '@data-tool'),
        (tools) => tools[0] === 'json-generator',
        'the tool named JSON first',
      );
    });

    it('narrows to the tools with @tools, a chip in the field', async () => {
      await palette.type('@tools');
      await press('Tab');
      await $(testid('palette-chip')).waitForExist({ timeout: 10_000 });
      await palette.type('json');

      const kinds = await readEach(testid('palette-option'), '@data-kind');
      expect(kinds.length).toBeGreaterThan(0);
      expect(new Set(kinds)).toEqual(new Set(['tool']));

      await palette.type('');
      await press('Backspace');
      await $(testid('palette-chip')).waitForExist({ reverse: true, timeout: 10_000 });
    });

    it('opens the request it found in its tab, in the HTTP area', async () => {
      await palette.type('signing keys');
      await row('data-request-id', requestId).$(testid('palette-open')).click();

      await $(testid('http-page')).waitForDisplayed({ timeout: 10_000 });
      await $(`${testid('http-tab')}[data-key="${requestId}"]`).waitForExist({ timeout: 10_000 });
      expect(await palette.input().isExisting()).toBe(false);
    });

    it('runs a command chosen after >', async () => {
      await emitGlobalAction('palette-here');
      await palette.input().waitForExist({ timeout: 10_000 });
      await palette.type('>');

      await row('data-command', 'area-tools').$(testid('palette-open')).click();

      await $(testid('tools-page')).waitForDisplayed({ timeout: 10_000 });
      await press('1', ['Control']);
      await $(testid('tools-page')).waitForExist({ reverse: true, timeout: 10_000 });
    });
  });

  it('re-registers the three accelerators and names the ones it lost', async () => {
    // A global accelerator is first-come-first-served across the machine and the loser
    // gets no error, so the command answers with what it could not take. Which of the
    // three is machine-dependent; that they are the only candidates is not.
    const asked = { palette: 'Ctrl+Alt+P', capture: 'Ctrl+Alt+V', newNote: 'Ctrl+Alt+N' };

    const first = await setShortcuts(asked);
    expect(first.err).toBeUndefined();
    for (const accelerator of first.ok ?? []) {
      expect(Object.values(asked)).toContain(accelerator);
    }

    // `register_shortcuts` drops all three and takes them again: the second call must
    // lose no more than the first, or it is unregistering its own.
    const second = await setShortcuts(asked);
    expect(second.ok).toEqual(first.ok);
  });
});
