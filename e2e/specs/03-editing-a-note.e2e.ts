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
/** TipTap hangs its editor on the surface it draws; the few members a scenario reads. */
interface RichEditor {
  view: { focus(): void };
  commands: { setTextSelection(at: number): boolean };
  state: { selection: { from: number; $from: { parent: { type: { name: string } } } } };
}

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
  describe('a Note', () => {
    const richTitle = 'Standup, formatted';
    let noteId = '';

    before(async () => {
      // A Note's content is never read for a language, so the draft can carry it.
      const content = '**Ship** it and `tag`\n\n- [ ] write the notes';
      noteId = (await bridge.createNote(draft({ spaceId, title: richTitle, content, kind: 'note' }))).id;
      await reloadCanvas();
      await canvas.waitForCard(richTitle);
    });

    it('reaches its card as words, without the Markdown', async () => {
      const view = await bridge.queryNotes(query({ search: richTitle }));

      expect(view.sections[0]?.notes[0]?.content).toBe('Ship it and tag\n☐ write the notes');
    });

    /** Its shape, read by Rust, and its kind where a snippet shows its format. */
    it('shows its outline on its card, under a Note badge', async () => {
      const card = await canvas.cardWithTitle(richTitle);

      expect(await card.$(testid('kind-badge')).isExisting()).toBe(true);
      const task = card.$(`${testid('note-card-outline')} .outline-item`);
      await task.waitForExist({ timeout: 10_000 });
      expect(await task.getText()).toContain('write the notes');
    });

    it('opens formatted, and a box ticked there is stored as Markdown', async () => {
      await canvas.openNote(richTitle);
      const bold = $(`${testid('editor-rich')} strong`);
      await bold.waitForExist({ timeout: 10_000 });
      expect(await bold.getText()).toBe('Ship');

      // Its kind where a snippet names its format, and its words — seven — in the footer.
      expect(await $(testid('editor-kind')).isExisting()).toBe(true);
      expect(await $(testid('choice-language')).isExisting()).toBe(false);
      expect(await $(testid('editor-stats')).getText()).toContain('7');

      await $(`${testid('editor-rich')} input[type="checkbox"]`).click();
      await editor.close();

      const stored = await eventually(
        async () => (await bridge.getNote(noteId)).content,
        (content) => content.includes('- [x] write the notes'),
        'the ticked box to reach the database',
      );
      expect(stored).toContain('**Ship** it and `tag`');
    });

    /** Coloured by the code field's grammars, stored as the fence it was given, and changed from a menu. */
    it('holds a code block in its language', async () => {
      const title = 'Checks after the switch';
      const content = 'Run it:\n\n```sql\nSELECT pg_is_in_recovery();\n```';
      const id = (await bridge.createNote(draft({ spaceId, title, content, kind: 'note' }))).id;
      await reloadCanvas();
      await canvas.waitForCard(title);

      await canvas.openNote(title);
      const keyword = $(`${testid('editor-rich')} pre .hljs-keyword`);
      await keyword.waitForExist({ timeout: 10_000 });
      expect(await keyword.getText()).toBe('SELECT');
      expect(await $(`${testid('editor-rich')} pre`).getAttribute('data-language')).toBe('SQL');

      // From the themed menu, and the caret is still in the block once it has chosen. Placed
      // through the editor: the driver's click reaches the element but moves no caret.
      await browser.execute((selector: string) => {
        const surface = document.querySelector(selector) as HTMLElement & { editor: RichEditor };
        surface.editor.view.focus();
        surface.editor.commands.setTextSelection(14);
      }, testid('editor-rich'));
      await $(testid('rich-code-language')).click();
      // The DOM's own click: the driver's focuses what it clicked afterwards, which a mouse never
      // does, and the entry leaves the page with the focus on it.
      await browser.execute(
        (selector: string) => (document.querySelector(selector) as HTMLElement).click(),
        `${testid('choice-panel-rich-code-language')} [data-option-id="py"]`,
      );
      await $(`${testid('editor-rich')} pre[data-language="PY"]`).waitForExist({ timeout: 5_000 });

      const caret = await browser.execute((selector: string) => {
        const surface = document.querySelector(selector) as HTMLElement & { editor: RichEditor };
        const selection = surface.editor.state.selection;
        return {
          at: selection.from,
          in: selection.$from.parent.type.name,
          focused: surface.contains(document.activeElement),
        };
      }, testid('editor-rich'));
      expect(caret).toEqual({ at: 14, in: 'codeBlock', focused: true });
      await editor.close();

      const stored = await eventually(
        async () => (await bridge.getNote(id)).content,
        (written) => written.includes('```py'),
        'the chosen language to reach the database',
      );
      expect(stored).toBe('Run it:\n\n```py\nSELECT pg_is_in_recovery();\n```');
      await bridge.deleteNotes([id]);
      await bridge.purgeNotes([id]);
      await reloadCanvas();
    });

    /** Code keeps its characters and gets its language: the note becomes a snippet. */
    it('turns into a snippet in the code field when code is pasted into it empty', async () => {
      const code = 'SELECT id, title FROM notes WHERE pinned = 1;';
      await canvas.createRichNote();
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
      expect(pasted?.kind).toBe('snippet');
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

  /** Detected in Rust, badged by a hue rule and coloured by a grammar: three places to forget. */
  it('recognises SCSS and GraphQL, and draws them as languages', async () => {
    const samples = [
      { title: 'Card styles', content: '$gap: 8px;\n\n.card {\n  padding: $gap;\n}', language: 'scss' },
      {
        title: 'Invoices query',
        content: 'query Invoices($id: ID!) {\n  customer(id: $id) {\n    name\n  }\n}',
        language: 'graphql',
      },
    ] as const;
    const ids = [];
    for (const sample of samples) {
      ids.push(
        (await bridge.createNote(draft({ spaceId, title: sample.title, content: sample.content }))).id,
      );
    }
    await reloadCanvas();

    for (const [index, sample] of samples.entries()) {
      await canvas.waitForCard(sample.title);
      expect((await bridge.getNote(ids[index]!)).language).toBe(sample.language);

      const badge = (await canvas.cardWithTitle(sample.title)).$('.lang-tag');
      expect(await badge.getText()).toBe(sample.language.toUpperCase());
      // A language with no `.lang-*` rule still draws its label, on no fill at all.
      expect((await badge.getCSSProperty('background-color')).value).not.toBe('rgba(0,0,0,0)');

      await canvas.openNote(sample.title);
      expect(await $('.editor-stack .line-content span[class^="hljs-"]').isExisting()).toBe(true);
      await editor.close();
    }

    await bridge.deleteNotes(ids);
    await bridge.purgeNotes(ids);
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
