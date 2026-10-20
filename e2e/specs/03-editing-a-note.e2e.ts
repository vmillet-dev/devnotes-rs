import { $, $$, browser, expect } from '@wdio/globals';

import { canvas } from '../pageobjects/canvas.page.js';
import { editor } from '../pageobjects/editor.page.js';
import { undoBar } from '../pageobjects/overlays.page.js';
import { spaces } from '../pageobjects/sidebar.page.js';
import { settings } from '../pageobjects/titlebar.page.js';
import {
  eventually,
  isInFront,
  press,
  reloadCanvas,
  setField,
  testid,
  viewportSize,
} from '../support/app.js';
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

  it('folds the header for the code, from note to note and in full screen', async () => {
    const other = 'Folded header, next note';
    const { id } = await bridge.createNote(draft({ spaceId, title: other, content: 'echo next' }));
    await reloadCanvas();

    await canvas.openNote(title);
    const unfolded = await editor.codeHeight();
    await editor.toggleHeader();
    expect(await editor.isHeaderFolded()).toBe(true);
    // Context, placement and attachments give way to one line: some 90 px back.
    const folded = await eventually(
      () => editor.codeHeight(),
      (height) => height > unfolded + 60,
      'the code to take the height back',
    );
    await editor.close();

    await canvas.openNote(other);
    expect(await editor.isHeaderFolded()).toBe(true);
    await editor.toggleFullscreen();
    await eventually(
      () => editor.codeHeight(),
      (height) => height > folded,
      'the code to grow with the full screen',
    );
    expect(await editor.isHeaderFolded()).toBe(true);
    await editor.toggleFullscreen();

    await editor.headerSummary().click();
    await $(testid('editor-placement')).waitForExist({ timeout: 5_000 });
    expect(await editor.isHeaderFolded()).toBe(false);
    await editor.close();

    await bridge.deleteNotes([id]);
    await bridge.purgeNotes([id]);
    await reloadCanvas();
  });

  /** Its labels give way first: the bar of a note with fields once pushed its close button out. */
  it('keeps the title bar whole down to the smallest window', async () => {
    const fields = 'Title bar of a note with fields';
    const { id } = await bridge.createNote(
      draft({ spaceId, title: fields, content: 'psql -h {{host}} -p {{port}}', language: 'sh' }),
    );
    const restore = await browser.getWindowSize();
    await reloadCanvas();
    await canvas.openNote(fields);

    for (const width of [1280, 1100, 900, 640]) {
      await browser.setWindowSize(width, 720);
      // A resize lands a few frames later: measured before, the old size passed for the new one.
      let last = -1;
      await eventually(
        () => browser.execute(() => window.innerWidth),
        (now) => {
          const settled = now === last;
          last = now;
          return settled && now <= width;
        },
        `the window at ${width} px`,
      );
      const layout = await eventually(
        () =>
          browser.execute(() => {
            const row = document.querySelector('.toolbar-row')!;
            const bar = document.querySelector('.overlay-toolbar')!.getBoundingClientRect();
            const close = document.querySelector('[data-testid="editor-close"]')!.getBoundingClientRect();
            const title = document.querySelector('[data-testid="editor-title"]')!.getBoundingClientRect();
            const meta = document.querySelector('.overlay-meta-row')!;
            return {
              closeInside: close.right <= bar.right - 8 && row.scrollWidth <= row.clientWidth + 1,
              title: Math.round(title.width),
              metaInside: meta.scrollWidth <= meta.clientWidth + 1,
            };
          }),
        (seen) => seen.closeInside && seen.metaInside && seen.title >= 120,
        `the title bar and the row under it whole at ${width} px`,
      );
      expect(layout.closeInside).toBe(true);
      expect(layout.metaInside).toBe(true);
      // Readable, not merely present: « Déployer la factur » was what the labels left at 1280.
      expect(layout.title).toBeGreaterThanOrEqual(width >= 900 ? 240 : 120);
    }

    await browser.setWindowSize(restore.width, restore.height);
    await editor.close();
    await bridge.deleteNotes([id]);
    await bridge.purgeNotes([id]);
    await reloadCanvas();
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

  it('picks a format by typing its name, Escape clearing the filter before the menu', async () => {
    const shown = () =>
      browser.execute(
        (selector: string) => {
          const entries = document.querySelectorAll<HTMLElement>(selector);
          return Array.from(entries, (entry) => entry.dataset['optionId'] ?? '').join(',');
        },
        `${testid('choice-panel-language')} ${testid('choice-option')}`,
      );
    const focused = () => browser.execute(() => document.activeElement?.getAttribute('data-testid') ?? '');

    await canvas.openNote(title);
    await $(testid('choice-language')).click();
    await $(testid('choice-filter')).waitForDisplayed({ timeout: 5_000 });
    await eventually(focused, (id) => id === 'choice-filter', 'the filter field to take the focus');

    await $(testid('choice-filter')).setValue('pyth');
    await eventually(shown, (ids) => ids === 'py', 'Python alone, found by its full name');

    await $(testid('choice-filter')).setValue('klingon');
    await $(testid('choice-empty')).waitForDisplayed({ timeout: 5_000 });

    await press('Escape');
    await eventually(
      shown,
      (ids) => ids.split(',').length > 20,
      'every format back once the filter is cleared',
    );
    expect(await $(testid('choice-filter')).getValue()).toBe('');

    await $(testid('choice-filter')).setValue('ELIX');
    await eventually(shown, (ids) => ids === 'ex', 'Elixir, case aside');
    await $(testid('choice-filter')).setValue('rust');
    await eventually(shown, (ids) => ids === 'rs', 'Rust alone');
    await press('Enter');
    await $(testid('choice-panel-language')).waitForExist({ reverse: true, timeout: 5_000 });
    await editor.close();

    expect((await reread())?.language).toBe('rs');

    // Closed after a filter was cleared: the first Escape empties it, the second closes the menu.
    await canvas.openNote(title);
    await $(testid('choice-language')).click();
    await $(testid('choice-filter')).waitForDisplayed({ timeout: 5_000 });
    await eventually(focused, (id) => id === 'choice-filter', 'the filter field to take the focus');
    await $(testid('choice-filter')).setValue('sql');
    await press('Escape');
    await press('Escape');
    await $(testid('choice-panel-language')).waitForExist({ reverse: true, timeout: 5_000 });
    expect(await $(testid('editor-body')).isExisting()).toBe(true);
    await editor.close();
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

  it('keeps a tag typed without Enter when Escape closes the editor', async () => {
    await canvas.openNote(title);
    await setField(testid('editor-tag-add'), 'urgent');
    await browser.keys(['Escape']);
    await $(testid('editor-title')).waitForExist({ reverse: true, timeout: 10_000 });

    await eventually(
      async () => (await reread())?.tags ?? [],
      (tags) => tags.includes('urgent'),
      'the tag typed into the field to be written on the way out',
    );

    await canvas.openNote(title);
    await editor.removeTag('urgent');
    await editor.close();
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

  /** Right after: the space chosen is remembered, and every later scenario reads all of them. */
  it('shows every space again through the "all spaces" row', async () => {
    await spaces.open();
    await spaces.allOption().click();
    // `null` is a choice, not a loading state: the note filed in `Ops` is still listed.
    await canvas.waitForCard(title);

    expect(await spaces.allOption().getAttribute('aria-current')).toBe('true');
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

    /** A todo list read as a Note's tasks, and back: its boxes go round unchanged. */
    it('converts a todo list into a Note whose tasks it holds, and back again', async () => {
      const title = 'Release steps';
      const items = [
        { text: 'Freeze the merges', done: true },
        { text: 'Ship the build', done: false },
      ];
      const id = (await bridge.createNote(draft({ spaceId, title, kind: 'checklist', items }))).id;
      await reloadCanvas();
      await canvas.openNote(title);

      await editor.convertTo('note');
      const tasks = () => $$(`${testid('editor-rich')} input[type="checkbox"]`).length;
      await eventually(tasks, (count) => count === 2, 'the items to become the Note’s tasks');
      expect(await $(testid('editor-rich')).getText()).toContain('Freeze the merges');
      expect((await bridge.getNote(id)).content).toBe(
        ['- [x] Freeze the merges', '- [ ] Ship the build'].join('\n'),
      );
      await undoBar.bar().waitForDisplayed({ timeout: 10_000 });

      await editor.convertTo('checklist');
      await eventually(
        () => editor.itemTexts(),
        (texts) => texts.join('|') === 'Freeze the merges|Ship the build',
        'the tasks to be items again',
      );
      expect(await editor.itemChecks()).toEqual([true, false]);
      await editor.close();
      expect((await bridge.getNote(id)).items).toEqual(items);
    });

    /** The code field's bar over the text: one replace for every « deploy », one Ctrl+Z for all. */
    it('replaces a word everywhere from Ctrl+H, and Ctrl+Z puts every one back', async () => {
      const title = 'Replaced everywhere';
      const content = '**Deploy** on Monday, deploy again on Friday.';
      const id = (await bridge.createNote(draft({ spaceId, title, content, kind: 'note' }))).id;
      await reloadCanvas();
      await canvas.openNote(title);
      const text = () => $(testid('editor-rich')).getText();

      await $(testid('editor-rich')).click();
      await press('h', ['Control']);
      await $(testid('search-replace-field')).waitForDisplayed({ timeout: 5_000 });
      await setField(testid('search-field'), 'deploy');
      await eventually(
        () => $(testid('search-count')).getText(),
        (count) => count.endsWith('2'),
        'both words found, the bold one too',
      );
      await setField(testid('search-replace-field'), 'ship');
      await $(testid('search-replace-all')).click();
      await eventually(text, (now) => !/deploy/i.test(now), 'every word replaced');
      expect(await $(`${testid('editor-rich')} strong`).getText()).toBe('ship');

      await $(testid('search-close')).click();
      await press('z', ['Control']);
      await eventually(
        text,
        (now) => now.includes('Deploy on Monday, deploy again'),
        'one Ctrl+Z to put them back',
      );
      await editor.close();

      expect((await bridge.getNote(id)).content).toBe(content);
      await bridge.deleteNotes([id]);
      await bridge.purgeNotes([id]);
    });

    /** Coloured by the code field's grammars, stored as the fence it was given, and changed from a menu. */
    it('holds a code block in its language', async () => {
      const title = 'Checks after the switch';
      const content = 'Run it:\n\n```sql\nSELECT pg_is_in_recovery();\n```';
      const id = (await bridge.createNote(draft({ spaceId, title, content, kind: 'note' }))).id;
      await reloadCanvas();
      await canvas.waitForCard(title);

      await canvas.openNote(title);
      const keyword = $(`${testid('editor-rich')} pre .tok-keyword`);
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

    /** Drawn by Mermaid under the CSP as it stands, and stored as the fence it was given. */
    it('draws a Mermaid block as a diagram under its code', async () => {
      const title = 'Login sequence';
      const content =
        'How it goes:\n\n```mermaid\nsequenceDiagram\n  Browser->>API: POST /login\n  API-->>Browser: 200 and a cookie\n```';
      const id = (await bridge.createNote(draft({ spaceId, title, content, kind: 'note' }))).id;
      await reloadCanvas();
      await canvas.waitForCard(title);

      await canvas.openNote(title);
      await $(`${testid('rich-diagram')} svg`).waitForExist({ timeout: 15_000 });
      expect(await $(`${testid('editor-rich')} pre`).getAttribute('data-language')).toBe('MERMAID');
      const drawn = await browser.execute(
        (selector: string) => document.querySelector(selector)?.textContent ?? '',
        `${testid('rich-diagram')} svg`,
      );
      expect(drawn).toContain('POST /login');
      await editor.close();

      expect((await bridge.getNote(id)).content).toBe(content);
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

  /** CodeMirror's own key handling, which keeps it in the editor's undo. */
  it('indents with Tab rather than leaving the code', async () => {
    const indented = 'Indented with Tab';
    const { id } = await bridge.createNote(
      draft({ spaceId, title: indented, content: 'echo hi', language: 'sh' }),
    );
    await reloadCanvas();
    await canvas.openNote(indented);
    await editor.focusCode();

    await press('Tab');

    expect(
      await eventually(
        () => editor.body(),
        (body) => body === '  echo hi',
        'the line indented',
      ),
    ).toBe('  echo hi');
    expect(await editor.codeHasFocus()).toBe(true);
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

  it('continues a wrapped line under its indentation, and scrolls it sideways unwrapped', async () => {
    const wrapped = 'Wrapped under its indentation';
    const query = 'https://api.example.com/v2/invoices?status=paid&customer=c_1042&page=';
    const content = `class Invoices {\n    readonly url = '${query.repeat(5)}';\n}`;
    const { id } = await bridge.createNote(draft({ spaceId, title: wrapped, content, language: 'ts' }));
    await settings.setCodeWrap(true);
    await reloadCanvas();

    await canvas.openNote(wrapped);
    const rows = await eventually(
      () => editor.wrappedRows('readonly url'),
      (measured) => measured.rowStarts.length > 1,
      'the long line on several rows',
    );
    // The first row keeps its indentation; the next start two characters right of its text.
    expect(rows.rowStarts[0]).toBeLessThan(rows.textLeft);
    expect((rows.rowStarts[1]! - rows.textLeft) / rows.charWidth).toBeCloseTo(2, 0);
    await editor.close();

    await settings.setCodeWrap(false);
    await canvas.openNote(wrapped);
    await eventually(
      () => editor.wrappedRows('readonly url'),
      (measured) => measured.rowStarts.length === 1,
      'the long line on one row',
    );
    expect(await editor.codeScrollsSideways()).toBe(true);
    await editor.close();

    await bridge.deleteNotes([id]);
    await bridge.purgeNotes([id]);
    await reloadCanvas();
  });

  /**
   * Detected in Rust, badged by its family's hue, and coloured by one grammar on the card and in
   * the editor alike: Terraform's is one of ours.
   */
  it('recognises SCSS, GraphQL and Terraform, and draws them as languages', async () => {
    const samples = [
      { title: 'Card styles', content: '$gap: 8px;\n\n.card {\n  padding: $gap;\n}', language: 'scss' },
      {
        title: 'Invoices query',
        content: 'query Invoices($id: ID!) {\n  customer(id: $id) {\n    name\n  }\n}',
        language: 'graphql',
      },
      {
        title: 'Assets bucket',
        content: 'resource "aws_s3_bucket" "assets" {\n  bucket = var.name\n}',
        language: 'tf',
        keyword: 'resource',
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
      // A badge with no hue still draws its label, on no fill at all.
      expect((await badge.getCSSProperty('background-color')).value).not.toBe('rgba(0,0,0,0)');

      // Coloured by its grammar, a chunk loaded with the first card in that language, and the
      // same classes in the editor.
      const card = await canvas.cardWithTitle(sample.title);
      await card.$('.card-snippet [class^="tok-"]').waitForExist({ timeout: 10_000 });
      if ('keyword' in sample) {
        expect(await card.$('.card-snippet .tok-keyword').getText()).toBe(sample.keyword);
      }
      await canvas.openNote(sample.title);
      await $(`${testid('editor-body')} .cm-line span[class^="tok-"]`).waitForExist({ timeout: 10_000 });
      if ('keyword' in sample) {
        expect(await $(`${testid('editor-body')} .cm-line .tok-keyword`).getText()).toBe(sample.keyword);
      }
      await editor.close();
    }

    await bridge.deleteNotes(ids);
    await bridge.purgeNotes(ids);
    await reloadCanvas();
  });

  /**
   * A package-lock pasted into a snippet once took seconds to open and a second per key: the
   * editor now draws what is on screen, and a key lands at once. Built and stored in the page:
   * WebDriver refuses a request body of two megabytes.
   */
  it('opens a snippet of two megabytes, drawing only what is on screen, and types into it', async () => {
    const big = 'Lockfile of two megabytes';
    const id = await browser.executeAsync(
      (base: Record<string, unknown>, done: (id: string) => void) => {
        const packages: Record<string, unknown> = {};
        for (let i = 0; i < 4200; i++) {
          const version = `${i % 9}.${i % 13}.${i % 7}`;
          packages[`node_modules/package-${i}`] = {
            version,
            resolved: `https://registry.npmjs.org/package-${i}/-/package-${i}-${version}.tgz`,
            integrity: `sha512-${'x'.repeat(86)}==`,
            dev: i % 2 === 0,
            license: 'MIT',
            dependencies: { [`package-${i + 1}`]: `^${i % 9}.0.0`, [`package-${i + 2}`]: `~${i % 5}.1.0` },
            engines: { node: '>=18' },
          };
        }
        const content = JSON.stringify({ name: 'app', lockfileVersion: 3, packages }, null, 2);
        const tauri = (window as unknown as Record<string, any>)['__TAURI__'];
        tauri.core
          .invoke('create_note', { draft: { ...base, content } })
          .then((note: { id: string }) => done(note.id));
      },
      draft({ spaceId, title: big, language: 'json' }) as unknown as Record<string, unknown>,
    );
    await reloadCanvas();

    await canvas.openNote(big);
    const lines = () => $$(`${testid('editor-body')} .cm-line`).length;
    await eventually(lines, (count) => count > 0, 'the first lines drawn');
    // Of some fifty thousand lines, what a window shows and a margin around it.
    expect(await lines()).toBeLessThan(500);

    await editor.focusCode();
    await browser.execute(() => document.execCommand('insertText', false, 'x'));
    await eventually(
      () => $(`${testid('editor-body')} .cm-line`).getText(),
      (first) => first.startsWith('x{'),
      'the key in the first line',
    );
    await editor.close();

    const stored = await eventually(
      () =>
        browser.executeAsync((noteId: string, done: (head: string) => void) => {
          const tauri = (window as unknown as Record<string, any>)['__TAURI__'];
          tauri.core
            .invoke('get_note', { id: noteId })
            .then((note: { content: string }) => done(`${note.content.slice(0, 2)} ${note.content.length}`));
        }, id),
      (head) => head.startsWith('x{'),
      'the edited lockfile saved whole',
    );
    expect(Number(stored.split(' ')[1])).toBeGreaterThan(1_500_000);
    await bridge.deleteNotes([id]);
    await bridge.purgeNotes([id]);
    await reloadCanvas();
  });

  it('finds and replaces in a snippet through a bar of its own', async () => {
    const searched = 'Searched through its own bar';
    const code = 'const total = 1;\nconst totalTax = total * 0.2;\nreturn total;';
    const { id } = await bridge.createNote(
      draft({ spaceId, title: searched, content: code, language: 'ts' }),
    );
    await reloadCanvas();
    await canvas.openNote(searched);
    await editor.focusCode();
    const count = () => $(testid('search-count')).getText();

    await press('f', ['Control']);
    await setField(testid('search-field'), 'total');
    await eventually(count, (text) => text === '1/4', 'four matches, the first selected');
    await $(testid('search-next')).click();
    await eventually(count, (text) => text === '2/4', 'the second match selected');

    const close = await $(testid('search-close')).getSize();
    expect(close.width).toBeGreaterThanOrEqual(24);
    expect(close.height).toBeGreaterThanOrEqual(24);

    await $(testid('search-replace-toggle')).click();
    await setField(testid('search-replace-field'), 'sum');
    await $(testid('search-replace-all')).click();
    await eventually(
      () => editor.body(),
      (body) => body === 'const sum = 1;\nconst sumTax = sum * 0.2;\nreturn sum;',
      'every match replaced',
    );

    // Escape closes the bar and hands the caret back; one undo takes the whole replace back.
    await press('Escape');
    await $(testid('search-field')).waitForExist({ reverse: true, timeout: 5_000 });
    expect(await editor.codeHasFocus()).toBe(true);
    await press('z', ['Control']);
    await eventually(
      () => editor.body(),
      (body) => body === code,
      'the replace undone in one step',
    );
    await editor.close();

    await bridge.deleteNotes([id]);
    await bridge.purgeNotes([id]);
    await reloadCanvas();
  });
});
