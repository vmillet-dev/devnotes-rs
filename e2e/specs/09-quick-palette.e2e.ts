import { browser, expect } from '@wdio/globals';

import { canvas } from '../pageobjects/canvas.page.js';
import { editor } from '../pageobjects/editor.page.js';
import { fieldsForm, palette } from '../pageobjects/overlays.page.js';
import { aboutMenu } from '../pageobjects/titlebar.page.js';
import { emitGlobalAction, isInFront, press, reloadCanvas, testid } from '../support/app.js';
import { bridge, draft, homeSpaceId } from '../support/bridge.js';

/**
 * ⚠️ The OS-level `Ctrl+Alt+P` is out of scope: WebDriver types into the WebView, not into
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

  it('says the corpus is empty rather than showing a bare list', async () => {
    await palette.type('');
    // Every note is a candidate with no query, so the empty row must not appear.
    expect(await palette.empty().isExisting()).toBe(false);
    expect(await palette.options().length).toBeGreaterThan(0);
  });

  /**
   * ⚠️ Enter used to copy and put the window away, where a click on the same row opened
   * the note — the first thing anyone does having found a result is press Enter.
   */
  it('opens the highlighted note in the editor on Enter', async () => {
    await palette.type('reset');
    await press('Enter');

    expect(await editor.isOpen()).toBe(true);
    expect(await editor.title()).toBe('Reset the dev database');
    await editor.close();
  });

  /**
   * ⚠️ The toast itself is out of reach — it is drawn by the operating system, after the
   * window has gone, and this run shares one window with every file after it. What the suite
   * can prove is the half that fails silently: a capability missing from
   * `capabilities/default.json` makes the plugin refuse at runtime and nothing else says so
   * (#285).
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

  /**
   * ⚠️ Asked of a snippet with fields on purpose: the copy path proper ends in
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

  /**
   * ⚠️ A click opens: it used to copy and hide the window, which reads as the application
   * crashing on the click — nothing on screen says anything was copied.
   */
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
   * ⚠️ The one way to a note while a help panel is up: the panel covers the whole page,
   * and a global shortcut comes from outside the application. The note used to open
   * behind it — `editor` was the bottom rung and `app` the one above.
   */
  it('opens a note in front of a help panel that was left open', async () => {
    await aboutMenu.openGettingStarted();

    await emitGlobalAction('palette');
    await palette.input().waitForExist({ timeout: 10_000 });
    await palette.type('reset');
    await press('Enter');

    await editor.title();
    expect(await isInFront(testid('editor-title'))).toBe(true);

    await editor.close();
    await browser.keys(['Escape']);
    await aboutMenu.gettingStarted().waitForExist({ reverse: true, timeout: 10_000 });
  });

  it('closes on Escape', async () => {
    await emitGlobalAction('palette');
    await palette.input().waitForExist({ timeout: 10_000 });
    await press('Escape');
    await palette.input().waitForExist({ reverse: true, timeout: 10_000 });
  });

  it('re-registers the three accelerators and names the ones it lost', async () => {
    // ⚠️ A global accelerator is first-come-first-served across the machine and the loser
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
