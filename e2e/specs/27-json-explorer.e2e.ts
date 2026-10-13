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
    const view = await bridge.exploreJson({
      text: EVENT,
      format: 'json',
      search: '',
      opening: { kind: 'initial' },
    });

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
    const view = await bridge.exploreJson({
      text: EVENT,
      format: 'json',
      search: 'MENSUEL',
      opening: { kind: 'initial' },
    });

    expect(view.matches).toEqual(['$.data.object.lines[0].description']);
  });

  it('says where a broken document breaks', async () => {
    const view = await bridge.exploreJson({
      text: '{\n  "a": 1,,\n}',
      format: 'json',
      search: '',
      opening: { kind: 'all' },
    });

    expect(view.error).toEqual({ reason: 'unexpectedCharacter', line: 2, column: 10, offset: 11 });
    expect(view.graph.nodes).toEqual([]);
  });

  describe('in the editor of a JSON snippet', () => {
    const TITLE = 'Webhook invoice.paid';
    const PRETTY = JSON.stringify(JSON.parse(EVENT), null, 2);
    let id = '';

    const selectedPath = () => $(testid('json-selected-path')).getText();

    before(async () => {
      // A space of its own, so what it asserts reads only what this file wrote.
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

    it('folds everything to the root and back, the selection falling back to its nearest ancestor', async () => {
      const nodes = () => $$(testid('json-node')).length;
      await $(`${testid('json-node')}[data-path="$.data.object"] ${testid('json-node-head')}`).click();
      expect(await selectedPath()).toBe('$.data.object');

      await $(testid('json-collapse-all')).click();
      await eventually(nodes, (count) => count === 1, 'the root alone');
      await eventually(selectedPath, (path) => path === '$.data', 'the selection on the row still drawn');

      await $(testid('json-open-all')).click();
      await eventually(nodes, (count) => count === 5, 'every node again');
      await eventually(selectedPath, (path) => path === '$.data.object', 'the selection back where it was');

      // The same pair from the keys, the focus staying on the graph from one answer to the next.
      await $(`${testid('json-node')}[data-path="$"] ${testid('json-node-head')}`).click();
      await press('ArrowLeft', ['Shift']);
      await eventually(nodes, (count) => count === 1, 'the root alone, from the keys');
      await press('ArrowRight', ['Shift']);
      await eventually(nodes, (count) => count === 5, 'every node again, from the keys');
    });

    it('moves a node with the pointer and with the keys, its edges following, and puts it back', async () => {
      const object = `${testid('json-node')}[data-path="$.data.object"]`;
      // In the layout's own units, which neither the pan nor the zoom changes.
      const place = () =>
        browser.execute((selector: string) => {
          const node = document.querySelector<HTMLElement>(selector)!;
          const edges = Array.from(document.querySelectorAll('.edges path'), (path) =>
            path.getAttribute('d'),
          );
          return {
            left: parseFloat(node.style.left),
            top: parseFloat(node.style.top),
            edges: edges.join('|'),
          };
        }, object);
      const start = await place();

      // Synthetic pointer events: the embedded WebDriver dispatches no pointer actions. Id 1 is
      // the mouse, the pointer `setPointerCapture` accepts.
      await browser.execute((selector: string) => {
        const head = document.querySelector(`${selector} [data-testid="json-node-head"]`)!;
        const ground = document.querySelector('[data-testid="json-graph"]')!;
        const box = head.getBoundingClientRect();
        const send = (target: Element, type: string, dx: number, dy: number) =>
          target.dispatchEvent(
            new PointerEvent(type, {
              bubbles: true,
              cancelable: true,
              button: 0,
              pointerId: 1,
              clientX: box.left + 10 + dx,
              clientY: box.top + 10 + dy,
            }),
          );
        send(head, 'pointerdown', 0, 0);
        send(ground, 'pointermove', 60, 24);
        send(ground, 'pointermove', 120, 48);
        send(ground, 'pointerup', 120, 48);
      }, object);
      const dropped = await eventually(
        place,
        (now) => now.left > start.left,
        'the node dropped further right',
      );
      expect(dropped.top).toBeGreaterThan(start.top);
      expect(dropped.edges).not.toBe(start.edges);

      await $(`${object} ${testid('json-node-head')}`).click();
      expect(await selectedPath()).toBe('$.data.object');
      await press('ArrowDown', ['Alt']);
      await eventually(place, (now) => now.top === dropped.top + 24, 'the node a line lower, from the keys');

      await $(testid('json-reset-layout')).click();
      await eventually(
        place,
        (now) => now.left === start.left && now.top === start.top && now.edges === start.edges,
        'the node back where Rust placed it',
      );
      expect(await $(testid('json-reset-layout')).isExisting()).toBe(false);
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

  describe('a YAML snippet', () => {
    const TITLE = 'Compose services';
    // A wide character before what is asserted, and a second document after it.
    const YAML = [
      'services:',
      '  api:',
      '    image: "api:1.4"',
      '    ports: [80, 443]',
      '  🦀: &db',
      '    image: postgres',
      '  replica: *db',
      '---',
      'second: document',
      '',
    ].join('\n');
    let id = '';

    before(async () => {
      const spaceId = (await bridge.createSpace({ name: 'YAML' })).id;
      id = (await bridge.createNote(draft({ spaceId, title: TITLE, content: YAML, language: 'yml' }))).id;
      await reloadCanvas();
      await canvas.waitForCard(TITLE);
    });

    after(async () => {
      await bridge.deleteNotes([id]);
      await bridge.purgeNotes([id]);
      await reloadCanvas();
    });

    it('is read into the same model, its offsets pointing into the YAML itself', async () => {
      const view = await bridge.exploreJson({
        text: YAML,
        format: 'yaml',
        search: '',
        opening: { kind: 'all' },
      });

      expect(view.error).toBeNull();
      expect(view.notices).toEqual(['firstDocumentOnly']);
      const api = view.graph.nodes.find((node) => node.path === '$.services.api')!;
      const image = api.rows.find((row) => row.key === 'image')!;
      expect(YAML.slice(image.span.start, image.span.end)).toBe('"api:1.4"');
      // The alias reads as what it names: the same rows as its anchor.
      const replica = view.graph.nodes.find((node) => node.label === 'replica')!;
      expect(replica.rows.map((row) => row.value)).toEqual(['"postgres"']);

      const broken = await bridge.exploreJson({
        text: 'a: 1\nb: [1, 2\n',
        format: 'yaml',
        search: '',
        opening: { kind: 'initial' },
      });
      expect(broken.error?.reason).toBe('malformed');
      expect(broken.error?.line).toBeGreaterThanOrEqual(2);
    });

    it('opens on the same tabs, valid, saying what it left out, and shows a value in its code', async () => {
      await canvas.openNote(TITLE);
      await $(testid('editor-json-validity')).waitForExist({ timeout: 10_000 });
      expect(await $(testid('editor-json-validity')).getAttribute('class')).not.toContain('invalid');

      await $(testid('editor-view-graph')).click();
      await $(`${testid('json-node')}[data-path="$.services.api"]`).waitForExist({ timeout: 10_000 });
      expect(await $(`${testid('json-notice')}[data-notice="firstDocumentOnly"]`).isExisting()).toBe(true);
      expect(await $(testid('json-format-note')).isExisting()).toBe(true);

      await $(`${testid('json-row')}[data-path="$.services.api.image"]`).click();
      await $(testid('json-show-in-code')).click();
      await $(testid('editor-body')).waitForExist({ timeout: 10_000 });
      const selection = await browser.execute(() => {
        const field = document.querySelector<HTMLTextAreaElement>('[data-testid="editor-body"]')!;
        return field.value.slice(field.selectionStart, field.selectionEnd);
      });
      expect(selection).toBe('"api:1.4"');
      await editor.close();
    });
  });
});
