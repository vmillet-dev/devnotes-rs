import { $, $$, browser, expect } from '@wdio/globals';

import { canvas } from '../pageobjects/canvas.page.js';
import { editor } from '../pageobjects/editor.page.js';
import { eventually, press, reloadCanvas, testid } from '../support/app.js';
import { bridge, draft } from '../support/bridge.js';

/**
 * What the visualiser draws is decided in Rust: a text in, nodes placed out. Through the
 * assembled application's bridge, since the command answers without a library lock.
 */
describe('Exploring a JSON document', () => {
  const EVENT = JSON.stringify({
    id: 'evt_1PqX4c',
    type: 'invoice.paid',
    data: { object: { amount_paid: 4900, lines: [{ description: 'Pro · mensuel' }] } },
  });

  before(async () => {
    await canvas.open();
  });

  it('answers the nodes, their rows and where each one sits', async () => {
    const view = await bridge.exploreJson({ text: EVENT, search: '', opening: { kind: 'initial' } });

    expect(view.error).toBeNull();
    expect(view.graph.nodes.map((node) => node.path)).toEqual([
      '$',
      '$.data',
      '$.data.object',
      '$.data.object.lines',
      '$.data.object.lines[0]',
    ]);
    const root = view.graph.nodes[0]!;
    expect(root.rows.map((row) => row.value)).toEqual(['"evt_1PqX4c"', '"invoice.paid"', '{ … }']);
    expect(EVENT.slice(root.rows[0]!.span.start, root.rows[0]!.span.end)).toBe('"evt_1PqX4c"');
    expect(view.graph.nodes[1]!.x).toBeGreaterThan(root.x + root.width);
    expect(view.stats).toEqual({ keys: 7, depth: 4, bytes: new TextEncoder().encode(EVENT).length });
  });

  it('finds a key or a value, folded like every search', async () => {
    const view = await bridge.exploreJson({ text: EVENT, search: 'MENSUEL', opening: { kind: 'initial' } });

    expect(view.matches).toEqual(['$.data.object.lines[0].description']);
  });

  it('says where a broken document breaks', async () => {
    const view = await bridge.exploreJson({ text: '{\n  "a": 1,,\n}', search: '', opening: { kind: 'all' } });

    expect(view.error).toEqual({ reason: 'unexpectedCharacter', line: 2, column: 10, offset: 11 });
    expect(view.graph.nodes).toEqual([]);
  });

  describe('in the editor of a JSON snippet', () => {
    const TITLE = 'Webhook invoice.paid';
    const PRETTY = JSON.stringify(JSON.parse(EVENT), null, 2);
    let id = '';

    const selectedPath = () => $(testid('json-selected-path')).getText();

    before(async () => {
      // A space of its own: the first launch's went with the library 24-forgotten-passphrase set aside.
      const spaceId = (await bridge.createSpace({ name: 'JSON' })).id;
      id = (await bridge.createNote(draft({ spaceId, title: TITLE, content: PRETTY, language: 'json' }))).id;
      await reloadCanvas();
      await canvas.waitForCard(TITLE);
    });

    after(async () => {
      await bridge.deleteNotes([id]);
      await bridge.purgeNotes([id]);
      await reloadCanvas();
    });

    it('says the draft parses, over its code', async () => {
      await canvas.openNote(TITLE);

      await $(testid('editor-json-validity')).waitForExist({ timeout: 10_000 });
      expect(await $(testid('editor-json-validity')).getAttribute('class')).not.toContain('invalid');
      expect(await $(testid('editor-view-code')).getAttribute('aria-selected')).toBe('true');
    });

    it('draws the document as nodes, and selects one with its path', async () => {
      await $(testid('editor-view-graph')).click();
      await $(testid('json-graph')).waitForExist({ timeout: 10_000 });

      expect(await $$(testid('json-node')).length).toBe(5);
      await $(`${testid('json-node')}[data-path="$.data.object"] ${testid('json-node-head')}`).click();
      expect(await selectedPath()).toBe('$.data.object');
      expect(await $(testid('json-crumbs')).getText()).toContain('object');
    });

    it('closes and opens a container from its row', async () => {
      const row = () => $(`${testid('json-row')}[data-path="$.data.object.lines"]`);
      await row().click();
      await eventually(
        () => $$(testid('json-node')).length,
        (count) => count === 3,
        'the lines to close',
      );

      // One level: what was open under it was closed with it.
      await row().click();
      await eventually(
        () => $$(testid('json-node')).length,
        (count) => count === 4,
        'the lines to open again',
      );
    });

    it('walks the nodes with the arrows', async () => {
      await $(`${testid('json-node')}[data-path="$"] ${testid('json-node-head')}`).click();
      await press('ArrowDown');

      expect(await selectedPath()).toBe('$.id');
    });

    it('finds a value, and goes back to it in the code', async () => {
      await $(testid('json-search')).setValue('mensuel');
      await eventually(
        () => $(testid('json-match-count')).getText(),
        (label) => label === '1',
        'one match',
      );
      await $(testid('json-match-next')).click();
      await eventually(
        selectedPath,
        (path) => path === '$.data.object.lines[0].description',
        'the match selected',
      );

      await $(testid('json-show-in-code')).click();
      await $(testid('editor-body')).waitForExist({ timeout: 10_000 });
      const selection = await browser.execute(() => {
        const field = document.querySelector<HTMLTextAreaElement>('[data-testid="editor-body"]')!;
        return field.value.slice(field.selectionStart, field.selectionEnd);
      });
      expect(selection).toBe('"Pro · mensuel"');
    });

    it('lists the same document as a tree that folds', async () => {
      await $(testid('editor-view-tree')).click();
      await $(testid('json-tree')).waitForExist({ timeout: 10_000 });
      const lines = await $$(testid('json-line')).length;

      await $(`${testid('json-line')}[data-path="$.data"]`).click();
      await eventually(
        () => $$(testid('json-line')).length,
        (count) => count < lines,
        'the tree to fold',
      );
    });

    it('says where a broken draft breaks', async () => {
      await $(testid('editor-view-code')).click();
      await editor.setBody('{"a": 1,,}');

      await eventually(
        () => $(testid('editor-json-validity')).getAttribute('class'),
        (value) => value?.includes('invalid') === true,
        'the draft to be refused',
      );
      await $(testid('editor-view-graph')).click();
      await $(testid('json-invalid')).waitForExist({ timeout: 10_000 });
      await editor.close();
    });
  });
});
