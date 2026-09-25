import { $, browser, expect } from '@wdio/globals';

import { canvas } from '../pageobjects/canvas.page.js';
import { editor } from '../pageobjects/editor.page.js';
import { spaces } from '../pageobjects/sidebar.page.js';
import { eventually, isInFront, press, reloadCanvas, testid, viewportSize } from '../support/app.js';
import { bridge, draft, homeSpaceId, query } from '../support/bridge.js';

/**
 * Every field goes through one `applyPatch`. What a unit spec cannot see is the wire: an
 * omitted key must stay omitted, and `targetSpaceId` is the one argument Tauri renames.
 */
describe('Editing a note', () => {
  // Not a sample note's title: `reread()` takes the first hit of a search, so a shared
  // title makes the assertions depend on which of the two sorts first.
  const title = 'Rollout under edit';
  let spaceId = '';
  let refugeId = '';

  before(async () => {
    await canvas.open();
    spaceId = await homeSpaceId();
    refugeId = (await bridge.createSpace({ name: 'Ops' })).id;
    await bridge.createNote(draft({ spaceId, title, content: 'kubectl rollout status' }));
    await reloadCanvas();
    await canvas.waitForCard(title);
  });

  async function reread() {
    const view = await bridge.queryNotes(query({ search: title }));
    return view.sections[0]?.notes[0];
  }

  it('writes title, body and source on the way out', async () => {
    await canvas.openNote(title);
    await editor.setBody('kubectl rollout restart deployment/api');
    await editor.setSource('runbooks/api.md');
    await editor.close();

    const note = await reread();
    expect(note?.content).toBe('kubectl rollout restart deployment/api');
    expect(note?.source).toBe('runbooks/api.md');
  });

  it('counts what the body holds, in the footer', async () => {
    await canvas.openNote(title);
    // Derived from the draft, so it is what says the editor is on the note that was opened.
    const footer = await editor.footer();
    expect(footer).toContain('1');
    await editor.close();
  });

  it('goes fullscreen and back, without losing the draft', async () => {
    await canvas.openNote(title);
    expect(await editor.isFullscreen()).toBe(false);

    const viewport = await viewportSize();
    const framed = await editor.panelSize();
    expect(framed.width).toBeLessThan(viewport.width);

    await editor.toggleFullscreen();
    expect(await editor.isFullscreen()).toBe(true);
    expect(await editor.body()).toBe('kubectl rollout restart deployment/api');

    // Measured, not asked: the button reports itself pressed before the panel has grown.
    // It fills the window under the titlebar, which stays in reach.
    const bar = await browser.execute(
      () => document.querySelector('.titlebar')!.getBoundingClientRect().height,
    );
    const full = await eventually(
      () => editor.panelSize(),
      ({ width }) => width >= viewport.width - 1,
      'the editor to fill the window',
    );
    expect(full.height).toBeGreaterThanOrEqual(viewport.height - bar - 1);
    expect(full.height).toBeLessThanOrEqual(viewport.height - bar + 1);

    await $(testid('file-menu')).click();
    await $(testid('file-preferences')).waitForExist({ timeout: 5_000 });
    expect(await isInFront(testid('file-preferences'))).toBe(true);
    // One Escape, one thing: the menu closes and the note stays open.
    await press('Escape');
    await $(testid('file-preferences')).waitForExist({ reverse: true, timeout: 5_000 });
    expect(await editor.isOpen()).toBe(true);

    await editor.toggleFullscreen();
    expect(await editor.isFullscreen()).toBe(false);
    await editor.close();
  });

  it('changes the language through the generated union', async () => {
    await canvas.openNote(title);
    await editor.setLanguage('sh');
    await editor.close();

    expect((await reread())?.language).toBe('sh');

    // Through the select, the patch, the column and back: the column stores the literal,
    // so a variant added to the Rust enum takes no migration to get here.
    await canvas.openNote(title);
    await editor.setLanguage('rs');
    await editor.close();

    expect((await reread())?.language).toBe('rs');
  });

  it('adds and removes a tag, normalised by Rust', async () => {
    await canvas.openNote(title);
    // `notes::model::normalize_tags` and nowhere else.
    await editor.addTag('#deploy');
    await editor.addTag('Deploy');
    expect(await editor.tags()).toEqual(['deploy']);
    await editor.close();
    expect((await reread())?.tags).toEqual(['deploy']);

    await canvas.openNote(title);
    await editor.removeTag('deploy');
    await editor.close();
    expect((await reread())?.tags).toEqual([]);
  });

  it('pins the note, and the card says so', async () => {
    await canvas.openNote(title);
    await editor.togglePin();
    expect(await editor.isPinned()).toBe(true);
    await editor.close();

    expect((await reread())?.pinned).toBe(true);
    const card = await canvas.cardWithTitle(title);
    expect(await card.$('[data-testid="note-card-pin"]').isExisting()).toBe(true);
  });

  it('sets a deadline at the end of the local day', async () => {
    await canvas.openNote(title);
    await editor.setDeadline('2030-06-15');
    await editor.close();

    const lifecycle = (await reread())?.lifecycle;
    expect(lifecycle?.kind).toBe('expires');

    // Read back in local time, and late in the day: midnight would make a note dated
    // today expired the moment it was saved.
    const at = new Date((lifecycle as { at: string }).at);
    expect(at.getFullYear()).toBe(2030);
    expect(at.getMonth()).toBe(5);
    expect(at.getDate()).toBe(15);
    expect(at.getHours()).toBeGreaterThan(12);
  });

  it('moves the note to another space, through the renamed argument', async () => {
    await canvas.moveNote(title, refugeId);

    const filed = await eventually(
      async () => (await reread())?.spaceId,
      (spaceId) => spaceId === refugeId,
      'the note to be filed in the refuge',
    );
    expect(filed).toBe(refugeId);
  });

  it('leaves the note reachable from the space it moved to', async () => {
    await spaces.open();
    await spaces.option(refugeId).click();
    await canvas.waitForCard(title);
  });

  /** Written in the rich editor, stored as Markdown, shown on a card as words. */
  describe('a Text note', () => {
    const richTitle = 'Standup, formatted';
    let noteId = '';

    before(async () => {
      noteId = (await bridge.createNote(draft({ spaceId, title: richTitle, content: '' }))).id;
      // A patch and not the draft: a draft's content is still read for a language.
      await bridge.updateNote(noteId, { content: '**Ship** it and `tag`\n\n- [ ] write the notes' });
      await reloadCanvas();
      await canvas.waitForCard(richTitle);
    });

    it('reaches its card as words, without the Markdown', async () => {
      const view = await bridge.queryNotes(query({ search: richTitle }));

      expect(view.sections[0]?.notes[0]?.content).toBe('Ship it and tag\n☐ write the notes');
    });

    it('opens formatted, and a box ticked there is stored as Markdown', async () => {
      await canvas.openNote(richTitle);
      const bold = $(`${testid('editor-rich')} strong`);
      await bold.waitForExist({ timeout: 10_000 });
      expect(await bold.getText()).toBe('Ship');

      await $(`${testid('editor-rich')} input[type="checkbox"]`).click();
      await editor.close();

      const stored = await eventually(
        async () => (await bridge.getNote(noteId)).content,
        (content) => content.includes('- [x] write the notes'),
        'the ticked box to reach the database',
      );
      expect(stored).toContain('**Ship** it and `tag`');
    });

    /** Code keeps its characters and gets its language: the note leaves for the code field. */
    it('hands a paste of code over to the code field, with its language', async () => {
      const code = 'SELECT id, title FROM notes WHERE pinned = 1;';
      await canvas.createSnippet();
      await $(testid('editor-rich')).waitForExist({ timeout: 10_000 });

      await browser.execute(
        (selector: string, text: string) => {
          const data = new DataTransfer();
          data.setData('text/plain', text);
          const event = new ClipboardEvent('paste', { clipboardData: data, bubbles: true, cancelable: true });
          document.querySelector(selector)!.dispatchEvent(event);
        },
        testid('editor-rich'),
        code,
      );

      await $(testid('editor-body')).waitForExist({ timeout: 10_000 });
      expect(await editor.body()).toBe(code);
      await editor.close();

      const pasted = (await bridge.queryNotes(query({ search: 'pinned = 1' }))).sections[0]?.notes[0];
      expect(pasted?.language).toBe('sql');
      // Untitled, and in every later file's canvas otherwise.
      await bridge.deleteNotes([pasted!.id]);
      await bridge.purgeNotes([pasted!.id]);
      await reloadCanvas();
    });
  });

  /** Through the WebView's own editing, which is what keeps it in the field's undo. */
  it('indents with Tab rather than leaving the code', async () => {
    const indented = 'Indented with Tab';
    const { id } = await bridge.createNote(
      draft({ spaceId, title: indented, content: 'echo hi', language: 'sh' }),
    );
    await reloadCanvas();
    await canvas.openNote(indented);
    await browser.execute((selector: string) => {
      const field = document.querySelector(selector) as HTMLTextAreaElement;
      field.focus();
      field.setSelectionRange(0, 0);
    }, testid('editor-body'));

    await press('Tab');

    expect(await editor.body()).toBe('  echo hi');
    expect(await browser.execute(() => document.activeElement?.getAttribute('data-testid'))).toBe(
      'editor-body',
    );
    await editor.close();
    const stored = await eventually(
      async () => (await bridge.getNote(id)).content,
      (content) => content === '  echo hi',
      'the indented body to reach the database',
    );
    expect(stored).toBe('  echo hi');
    // In every later file's canvas otherwise.
    await bridge.deleteNotes([id]);
    await bridge.purgeNotes([id]);
    await reloadCanvas();
  });

  it('shows every space again through the "all spaces" row', async () => {
    await spaces.open();
    await spaces.allOption().click();
    // `null` is a choice, not a loading state: the note filed in `Ops` is still listed.
    await canvas.waitForCard(title);

    expect(await spaces.allOption().getAttribute('aria-current')).toBe('true');
  });
});
