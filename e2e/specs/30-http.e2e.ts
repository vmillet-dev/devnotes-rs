import { createServer } from 'node:http';
import type { AddressInfo } from 'node:net';

import { $, $$, browser, expect } from '@wdio/globals';

import type { HttpNode, HttpRequestDraft, HttpTree } from '@core/ipc/bindings';

import { canvas } from '../pageobjects/canvas.page.js';
import { eventually, press, readEach, setField, testid, viewportSize } from '../support/app.js';
import { bridge } from '../support/bridge.js';
import { echoSocket } from '../support/echo-socket.js';

/** The tree as names, a folder's children in brackets: `API[Auth[Login] Factures]`. */
function outline(tree: HttpTree): string[] {
  const nodes = (children: HttpNode[]): string =>
    children
      .map((node) =>
        node.kind === 'folder' ? `${node.folder.name}[${nodes(node.children)}]` : node.request.name,
      )
      .join(' ');
  return tree.collections.map((node) => `${node.collection.name}[${nodes(node.children)}]`);
}

const draft = (collectionId: string, folderId: string | null, name: string): HttpRequestDraft => ({
  collectionId,
  folderId,
  name,
  kind: 'http',
  method: 'GET',
  document: { url: `https://api.exemple.fr/${name}`, description: '' },
});

describe('HTTP collections', () => {
  before(async () => {
    await canvas.open();
  });

  it('keeps a collection in the library: built, reordered, counted and deleted with what it holds', async () => {
    const api = await bridge.createHttpCollection('API Paiements');
    const auth = await bridge.createHttpFolder(api.id, null, 'Auth');
    await bridge.createHttpRequest(draft(api.id, auth.id, 'Login'));
    const factures = await bridge.createHttpFolder(api.id, null, 'Factures');
    const list = await bridge.createHttpRequest(draft(api.id, factures.id, 'Lister les factures'));
    await bridge.createHttpRequest(draft(api.id, factures.id, 'Une facture'));

    expect(outline(await bridge.httpTree())).toContain(
      'API Paiements[Auth[Login] Factures[Lister les factures Une facture]]',
    );

    await bridge.moveHttpItem(
      { kind: 'request', id: list.id },
      { collectionId: api.id, folderId: auth.id, index: 0 },
    );
    expect(outline(await bridge.httpTree())).toContain(
      'API Paiements[Auth[Lister les factures Login] Factures[Une facture]]',
    );

    expect(await bridge.countHttpContents({ kind: 'collection', id: api.id })).toEqual({
      folders: 2,
      requests: 3,
    });
    await bridge.deleteHttpItem({ kind: 'collection', id: api.id });
    const left = (await bridge.httpTree()).collections.map((node) => node.collection.id);
    expect(left).not.toContain(api.id);
  });

  describe('the rail', () => {
    const named = async (name: string) => {
      const rows = await $$(testid('http-node')).getElements();
      for (const row of rows) {
        if ((await row.$(testid('http-node-name')).getText()).endsWith(name)) return row;
      }
      return undefined;
    };
    const rowNames = () => readEach(`${testid('http-node')} ${testid('http-node-name')}`, 'text');
    const choose = async (row: WebdriverIO.Element, action: string) => {
      await row.$(testid('http-node-menu')).click();
      await $(`${testid('http-node-action')}[data-action="${action}"]`).click();
    };
    const create = async (text: string) => {
      await setField(testid('http-create-input'), text);
      await $(testid('http-create-submit')).click();
    };

    after(async () => {
      await press('1', ['Control']);
      await $(testid('http-page')).waitForExist({ reverse: true, timeout: 10_000 });
    });

    it('builds a collection from the rail: a folder, two requests, one moved above the other', async () => {
      await press('3', ['Control']);
      await $(testid('http-page')).waitForDisplayed({ timeout: 10_000 });
      if (!(await $(testid('http-rail')).isExisting())) await press('b', ['Control']);
      await $(testid('http-rail')).waitForDisplayed({ timeout: 10_000 });

      await $(testid('http-collection-create-open')).click();
      await create('Boutique');
      await eventually(rowNames, (names) => names.includes('Boutique'), 'the collection');

      await choose((await named('Boutique'))!, 'newFolder');
      await create('Commandes');
      await eventually(rowNames, (names) => names.includes('Commandes'), 'the folder');

      for (const request of ['Lister', 'Annuler']) {
        await choose((await named('Commandes'))!, 'newRequest');
        await create(request);
        await eventually(rowNames, (names) => names.some((name) => name.endsWith(request)), request);
      }
      await eventually(
        () => $(testid('http-request-name')).getValue(),
        (text) => text === 'Annuler',
        'the new request opened',
      );

      const annuler = (await named('Annuler'))!;
      await annuler.$(testid('http-node-name')).click();
      await press('ArrowUp', ['Alt']);
      await eventually(
        rowNames,
        (names) =>
          names.findIndex((name) => name.endsWith('Annuler')) <
          names.findIndex((name) => name.endsWith('Lister')),
        'Annuler above Lister',
      );
      expect(await annuler.$(testid('http-node-badge')).getText()).toBe('GET');
    });

    it('folds a folder and keeps it folded, then deletes the collection after saying what goes', async () => {
      await (await named('Commandes'))!.$(testid('http-node-twisty')).click();
      await eventually(
        rowNames,
        (names) => !names.some((name) => name.endsWith('Lister')),
        'the folder folded',
      );

      await choose((await named('Boutique'))!, 'delete');
      await $(testid('http-delete-count')).waitForDisplayed({ timeout: 10_000 });
      expect((await $(testid('http-delete-count')).getText()).replace(/\D/g, '')).toBe('12');
      await $(testid('http-delete-submit')).click();
      await eventually(rowNames, (names) => !names.includes('Boutique'), 'the collection gone');
    });
  });

  describe('a request in its tab', () => {
    after(async () => {
      await press('1', ['Control']);
      await $(testid('http-page')).waitForExist({ reverse: true, timeout: 10_000 });
    });

    it('keeps the query and its table in step, takes a header, and saves on Ctrl+S', async () => {
      const api = await bridge.createHttpCollection('Facturation');
      const created = await bridge.createHttpRequest(draft(api.id, null, 'Factures'));
      await press('3', ['Control']);
      await $(testid('http-page')).waitForDisplayed({ timeout: 10_000 });
      if (!(await $(testid('http-rail')).isExisting())) await press('b', ['Control']);
      await $(`${testid('http-node')}[data-id="${created.id}"] ${testid('http-node-name')}`).click();
      await eventually(
        () => $(testid('http-request-name')).getValue(),
        (name) => name === 'Factures',
        'the request in its tab',
      );

      await setField(testid('http-url'), '{{baseUrl}}/invoices?page=1&limit=50');
      await eventually(
        () => readEach(`${testid('http-params-table')} ${testid('http-kv-key')}`, 'value'),
        (keys) => keys.join() === 'page,limit,',
        'the table read from the query',
      );
      expect(await readEach(testid('http-url-variable'), 'text')).toEqual(['{{baseUrl}}']);
      expect(await $(testid('http-tab-dirty')).isExisting()).toBe(true);

      await $(`${testid('http-section')}[data-section="headers"]`).click();
      await setField(`${testid('http-headers-table')} ${testid('http-kv-key')}`, 'Accept');
      await press('s', ['Control']);
      await $(testid('http-tab-dirty')).waitForExist({ reverse: true, timeout: 10_000 });

      const saved = await bridge.httpRequest(created.id);
      expect(saved.document.url).toBe('{{baseUrl}}/invoices?page=1&limit=50');
      expect(saved.document.params?.map((row) => row.key)).toEqual(['page', 'limit']);
      expect(saved.document.headers?.map((row) => row.key)).toEqual(['Accept']);

      const tab = `${testid('http-tab')}[data-key="${created.id}"]`;
      await $(`${tab} ${testid('http-tab-close')}`).click();
      await $(tab).waitForExist({ reverse: true, timeout: 10_000 });
      await bridge.deleteHttpItem({ kind: 'collection', id: api.id });
    });

    /**
     * Held to the width of the fields above them, these sentences broke one word before
     * their end, beside a thousand pixels of empty panel.
     */
    it('writes the sentences under Auth and Réglages on one line of a wide window', async () => {
      /** The lines a text is laid out on: one rectangle a line box. */
      const linesOf = (selector: string): Promise<number> =>
        browser.execute((target: string) => {
          const range = document.createRange();
          range.selectNodeContents(document.querySelector(target)!);
          return new Set([...range.getClientRects()].map((box) => Math.round(box.top))).size;
        }, selector);

      await press('3', ['Control']);
      await $(testid('http-page')).waitForDisplayed({ timeout: 10_000 });
      const restore = await browser.getWindowSize();
      await browser.setWindowSize(1600, 900);

      try {
        await eventually(viewportSize, (size) => size.width > 1500, 'a wide window');
        await $(testid('http-tab-new')).click();
        await $(`${testid('http-section')}[data-section="auth"]`).click();
        await $(testid('http-auth-inherited')).waitForDisplayed({ timeout: 10_000 });
        expect(await linesOf(testid('http-auth-inherited'))).toBe(1);

        await $(`${testid('http-section')}[data-section="settings"]`).click();
        await $(testid('http-transport-verifyTls')).click();
        await $(`${testid('choice-option')}[data-option-id="off"]`).click();
        await $(testid('http-transport-insecure')).waitForDisplayed({ timeout: 10_000 });
        expect(await linesOf(testid('http-transport-insecure'))).toBe(1);
      } finally {
        await browser.setWindowSize(restore.width, restore.height);
      }

      await $(`${testid('http-tab')}.on ${testid('http-tab-close')}`).click();
      await $(testid('http-close-discard')).click();
      await $(testid('http-close-dialog')).waitForExist({ reverse: true, timeout: 10_000 });
    });

    it('inherits the auth its collection sets, takes a JSON body, and keeps both', async () => {
      const api = await bridge.createHttpCollection('Paiements');
      const created = await bridge.createHttpRequest(draft(api.id, null, 'Payer'));
      await press('1', ['Control']);
      await $(testid('http-page')).waitForExist({ reverse: true, timeout: 10_000 });
      await press('3', ['Control']);
      await $(testid('http-page')).waitForDisplayed({ timeout: 10_000 });
      if (!(await $(testid('http-rail')).isExisting())) await press('b', ['Control']);

      await $(`${testid('http-node')}[data-id="${api.id}"] ${testid('http-node-menu')}`).click();
      await $(`${testid('http-node-action')}[data-action="settings"]`).click();
      await $(testid('http-settings-dialog')).waitForDisplayed({ timeout: 10_000 });
      await $(testid('http-auth-kind')).click();
      await $(`${testid('choice-option')}[data-option-id="bearer"]`).click();
      await setField(testid('http-auth-secret'), '{{accessToken}}');
      await $(testid('http-settings-save')).click();
      await $(testid('http-settings-dialog')).waitForExist({ reverse: true, timeout: 10_000 });

      await $(`${testid('http-node')}[data-id="${created.id}"] ${testid('http-node-name')}`).click();
      await $(`${testid('http-section')}[data-section="auth"]`).click();
      await eventually(
        () => $(testid('http-auth-inherited')).getText(),
        (text) => text.includes('{{accessToken}}') && text.includes('Paiements'),
        'the bearer inherited from the collection',
      );

      await $(`${testid('http-section')}[data-section="body"]`).click();
      await $(`${testid('segmented-http-body')} [data-segment-id="json"]`).click();
      await setField(testid('http-body-text'), '{"amount": 4900}');
      await $(testid('http-body-valid')).waitForDisplayed({ timeout: 10_000 });
      await $(`${testid('http-section')}[data-section="headers"]`).click();
      await eventually(
        () => readEach(testid('http-implied-header'), 'text'),
        (rows) => rows.some((row) => row.includes('application/json')),
        'the Content-Type the body implies',
      );

      await press('s', ['Control']);
      await $(testid('http-tab-dirty')).waitForExist({ reverse: true, timeout: 10_000 });
      const saved = await bridge.httpRequest(created.id);
      expect(saved.document.body).toEqual({ kind: 'json', text: '{"amount": 4900}' });
      expect(saved.document.auth).toEqual({ kind: 'inherit' });

      const tab = `${testid('http-tab')}[data-key="${created.id}"]`;
      await $(`${tab} ${testid('http-tab-close')}`).click();
      await $(tab).waitForExist({ reverse: true, timeout: 10_000 });
      await bridge.deleteHttpItem({ kind: 'collection', id: api.id });
    });

    /**
     * A menu's panel used to be cut by the editor it opened in, which scrolls: on a short
     * window the response pane hid all but its first entry.
     */
    it('opens the auth menu whole on a short window, down to its last entry', async () => {
      /**
       * Where a panel is and how wide: out of the flow, nothing holds it to its trigger's
       * width any more, and its entries were once measured side by side — the whole window.
       */
      const panelUnder = (trigger: string, panel: string) =>
        browser.execute(
          (triggerSelector: string, panelSelector: string) => {
            const from = document.querySelector(triggerSelector)!.getBoundingClientRect();
            const box = document.querySelector(panelSelector)!.getBoundingClientRect();
            return {
              atTrigger: Math.abs(box.left - from.left) <= 1,
              narrow: box.width <= 320,
            };
          },
          testid(trigger),
          testid(panel),
        );

      const api = await bridge.createHttpCollection('Menus');
      const created = await bridge.createHttpRequest(draft(api.id, null, 'Courte'));
      await press('1', ['Control']);
      await $(testid('http-page')).waitForExist({ reverse: true, timeout: 10_000 });
      await press('3', ['Control']);
      await $(testid('http-page')).waitForDisplayed({ timeout: 10_000 });
      if (!(await $(testid('http-rail')).isExisting())) await press('b', ['Control']);
      const restore = await browser.getWindowSize();
      await browser.setWindowSize(1100, 500);

      try {
        // The resize itself closes an open menu: settled before one is opened.
        await eventually(viewportSize, (size) => size.height <= 500 && size.height > 420, 'a short window');
        await $(`${testid('http-node')}[data-id="${created.id}"] ${testid('http-node-name')}`).click();
        await $(`${testid('http-section')}[data-section="auth"]`).click();
        await $(testid('http-auth-kind')).click();
        const options = `${testid('choice-panel-http-auth')} ${testid('choice-option')}`;
        await $(options).waitForDisplayed({ timeout: 10_000 });

        // Hit-tested at its centre: an entry under the response pane is in the page and takes no click.
        const last = await browser.execute((selector: string) => {
          const entries = document.querySelectorAll<HTMLElement>(selector);
          const entry = entries[entries.length - 1]!;
          const box = entry.getBoundingClientRect();
          const hit = document.elementFromPoint(box.left + box.width / 2, box.top + box.height / 2);
          return {
            id: entry.dataset['optionId'],
            inWindow: box.top >= 0 && box.bottom <= window.innerHeight,
            onTop: hit !== null && entry.contains(hit),
          };
        }, options);
        expect(last).toEqual({ id: 'apiKey', inWindow: true, onTop: true });
        expect(await panelUnder('http-auth-kind', 'choice-panel-http-auth')).toEqual({
          atTrigger: true,
          narrow: true,
        });

        await $(`${options}[data-option-id="apiKey"]`).click();
        await eventually(
          () => $(testid('http-auth')).getAttribute('data-kind'),
          (kind) => kind === 'apiKey',
          'the last entry of the menu to be chosen',
        );

        // The longest menu of the editor: its entries side by side would be 500px wide.
        await $(testid('http-method')).click();
        await $(testid('choice-panel-http-method')).waitForDisplayed({ timeout: 10_000 });
        expect(await panelUnder('http-method', 'choice-panel-http-method')).toEqual({
          atTrigger: true,
          narrow: true,
        });
        await press('Escape');
        await $(testid('choice-panel-http-method')).waitForExist({ reverse: true, timeout: 10_000 });
      } finally {
        await browser.setWindowSize(restore.width, restore.height);
      }

      const tab = `${testid('http-tab')}[data-key="${created.id}"]`;
      await $(`${tab} ${testid('http-tab-close')}`).click();
      await $(testid('http-close-discard')).click();
      await $(tab).waitForExist({ reverse: true, timeout: 10_000 });
      await bridge.deleteHttpItem({ kind: 'collection', id: api.id });
    });

    it('sends the request from Rust on Ctrl+Enter and shows its status, time, size and body', async () => {
      const heard: string[] = [];
      const server = createServer((request, response) => {
        heard.push(`${request.method} ${request.url}`);
        response.writeHead(201, { 'Content-Type': 'application/json' });
        response.end('{"id":7}');
      });
      await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve));
      const { port } = server.address() as AddressInfo;
      try {
        const api = await bridge.createHttpCollection('Local');
        const created = await bridge.createHttpRequest({
          ...draft(api.id, null, 'Créer'),
          method: 'POST',
          document: { url: `http://127.0.0.1:${port}/users?page=1`, description: '' },
        });
        await press('1', ['Control']);
        await $(testid('http-page')).waitForExist({ reverse: true, timeout: 10_000 });
        await press('3', ['Control']);
        await $(testid('http-page')).waitForDisplayed({ timeout: 10_000 });
        if (!(await $(testid('http-rail')).isExisting())) await press('b', ['Control']);
        await $(`${testid('http-node')}[data-id="${created.id}"] ${testid('http-node-name')}`).click();
        await $(testid('http-response-idle')).waitForDisplayed({ timeout: 10_000 });

        await press('Enter', ['Control']);
        await eventually(
          () => $(testid('http-response-status')).getText(),
          (text) => text === '201 Created',
          'the status of the answer',
        );
        expect(await $(testid('http-response-pretty')).getText()).toContain('"id": 7');
        await $(`${testid('segmented-http-response-view')} [data-segment-id="raw"]`).click();
        expect(await $(testid('http-response-body')).getText()).toBe('{"id":7}');
        expect(await $(testid('http-response-size')).isDisplayed()).toBe(true);
        expect(await $(testid('http-response-time')).isDisplayed()).toBe(true);
        expect(heard).toEqual(['POST /users?page=1']);

        await setField(testid('http-url'), '{{baseUrl}}/users');
        await $(testid('http-send')).click();
        await $(testid('http-response-failure')).waitForDisplayed({ timeout: 10_000 });
        expect(heard).toHaveLength(1);

        await $(testid('http-history-open')).click();
        await eventually(
          () => readEach(testid('http-history-status'), 'text'),
          (statuses) => statuses.join() === '201',
          'the one send that left, in the history',
        );
        await $(testid('http-history-item')).click();
        await eventually(
          () => $(testid('http-response-status')).getText(),
          (text) => text === '201 Created',
          'the entry read like a response',
        );
        await $(testid('http-history-open')).click();
        await $(testid('http-history')).waitForExist({ reverse: true, timeout: 10_000 });

        const tab = `${testid('http-tab')}[data-key="${created.id}"]`;
        await $(`${tab} ${testid('http-tab-close')}`).click();
        await $(testid('http-close-discard')).click();
        await $(tab).waitForExist({ reverse: true, timeout: 10_000 });
        await bridge.deleteHttpItem({ kind: 'collection', id: api.id });
      } finally {
        server.close();
      }
    });

    /**
     * The sentences stated the defaults, « 30 secondes » and « plus de 10 », for a request
     * cut after one second or stopped at its first redirect.
     */
    it('says a timeout and a redirect too many with the value the request was sent with', async () => {
      const server = createServer((request, response) => {
        if (request.url === '/slow') return; // never answered: the delay runs out
        response.writeHead(302, { Location: '/around' });
        response.end();
      });
      await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve));
      const { port } = server.address() as AddressInfo;
      try {
        const api = await bridge.createHttpCollection('Limites');
        await bridge.saveHttpSettings(
          { kind: 'collection', id: api.id },
          { auth: { kind: 'inherit' }, headers: [], transport: { timeoutMs: 400 } },
        );
        const slow = await bridge.createHttpRequest({
          ...draft(api.id, null, 'Lente'),
          document: { url: `http://127.0.0.1:${port}/slow`, description: '' },
        });
        const around = await bridge.createHttpRequest({
          ...draft(api.id, null, 'En rond'),
          document: {
            url: `http://127.0.0.1:${port}/around`,
            description: '',
            transport: { maxRedirects: 1 },
          },
        });
        await press('1', ['Control']);
        await $(testid('http-page')).waitForExist({ reverse: true, timeout: 10_000 });
        await press('3', ['Control']);
        await $(testid('http-page')).waitForDisplayed({ timeout: 10_000 });
        if (!(await $(testid('http-rail')).isExisting())) await press('b', ['Control']);
        const failure = async () => ({
          key: await $(testid('http-response-failure')).getAttribute('data-key'),
          text: await $(testid('http-response-failure')).getText(),
        });

        await $(`${testid('http-node')}[data-id="${slow.id}"] ${testid('http-node-name')}`).click();
        await $(testid('http-response-idle')).waitForDisplayed({ timeout: 10_000 });
        await $(testid('http-send')).click();
        await eventually(
          failure,
          ({ key, text }) =>
            key === 'errors.httpTimeout.collection' && text.includes('400 ms') && text.includes('Limites'),
          'the delay that ran out, and the collection that set it',
        );

        await $(`${testid('http-section')}[data-section="settings"]`).click();
        await setField(testid('http-transport-timeoutMs'), '1000');
        await $(testid('http-send')).click();
        await eventually(
          failure,
          ({ key, text }) => key === 'errors.httpTimeout.request' && text.includes('1 s'),
          "the request's own delay, in seconds",
        );

        await $(`${testid('http-node')}[data-id="${around.id}"] ${testid('http-node-name')}`).click();
        await $(testid('http-response-idle')).waitForDisplayed({ timeout: 10_000 });
        await $(testid('http-send')).click();
        // « plus d'une », « more than one »: said in words, and never as the default's 10.
        await eventually(
          failure,
          ({ key, text }) => key === 'errors.httpTooManyRedirects.request' && !/\d/.test(text),
          'the limit the request set, not the default',
        );

        for (const request of [slow, around]) {
          const tab = `${testid('http-tab')}[data-key="${request.id}"]`;
          await $(`${tab} ${testid('http-tab-close')}`).click();
          if (request === slow) await $(testid('http-close-discard')).click();
          await $(tab).waitForExist({ reverse: true, timeout: 10_000 });
        }
        await bridge.deleteHttpItem({ kind: 'collection', id: api.id });
      } finally {
        server.close();
        server.closeAllConnections();
      }
    });

    it('posts a GraphQL query as JSON, and reads its errors apart from its data', async () => {
      const bodies: string[] = [];
      const server = createServer((request, response) => {
        let body = '';
        request.on('data', (chunk: Buffer) => (body += chunk.toString()));
        request.on('end', () => {
          bodies.push(body);
          response.writeHead(200, { 'Content-Type': 'application/json' });
          response.end('{"data":{"products":[]},"errors":[{"message":"Partial","path":["products"]}]}');
        });
      });
      await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve));
      const { port } = server.address() as AddressInfo;
      try {
        const api = await bridge.createHttpCollection('Catalogue');
        const created = await bridge.createHttpRequest({
          ...draft(api.id, null, 'Produits'),
          kind: 'graphql',
          document: {
            url: `http://127.0.0.1:${port}/graphql`,
            description: '',
            graphql: { query: '{ products { id } }' },
          },
        });
        await press('1', ['Control']);
        await $(testid('http-page')).waitForExist({ reverse: true, timeout: 10_000 });
        await press('3', ['Control']);
        await $(testid('http-page')).waitForDisplayed({ timeout: 10_000 });
        if (!(await $(testid('http-rail')).isExisting())) await press('b', ['Control']);
        await $(`${testid('http-node')}[data-id="${created.id}"] ${testid('http-node-name')}`).click();
        await $(testid('http-graphql-query')).waitForDisplayed({ timeout: 10_000 });

        await press('Enter', ['Control']);
        await $(`${testid('http-response-section')}[data-section="errors"]`).waitForDisplayed({
          timeout: 10_000,
        });
        expect(JSON.parse(bodies[0]!)).toEqual({ query: '{ products { id } }' });
        expect(await $(testid('http-response-pretty')).getText()).toContain('"products": []');
        await $(`${testid('http-response-section')}[data-section="errors"]`).click();
        expect(await $(testid('http-response-error')).getText()).toContain('Partial');

        const tab = `${testid('http-tab')}[data-key="${created.id}"]`;
        await $(`${tab} ${testid('http-tab-close')}`).click();
        await $(tab).waitForExist({ reverse: true, timeout: 10_000 });
        await bridge.deleteHttpItem({ kind: 'collection', id: api.id });
      } finally {
        server.close();
      }
    });

    /** A still « Envoi en cours… » made a slow server and a frozen application look the same. */
    it('shows a request on its way: a spinner, the time going by, its tab marked', async () => {
      let answer: () => void = () => undefined;
      const held = new Promise<void>((resolve) => (answer = resolve));
      const server = createServer((_, response) => {
        void held.then(() => {
          response.writeHead(200, { 'Content-Type': 'text/plain' });
          response.end('late');
        });
      });
      await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve));
      const { port } = server.address() as AddressInfo;
      try {
        const api = await bridge.createHttpCollection('Lenteur');
        const created = await bridge.createHttpRequest({
          ...draft(api.id, null, 'Tarde'),
          document: { url: `http://127.0.0.1:${port}/late`, description: '' },
        });
        await press('1', ['Control']);
        await $(testid('http-page')).waitForExist({ reverse: true, timeout: 10_000 });
        await press('3', ['Control']);
        await $(testid('http-page')).waitForDisplayed({ timeout: 10_000 });
        if (!(await $(testid('http-rail')).isExisting())) await press('b', ['Control']);
        await $(`${testid('http-node')}[data-id="${created.id}"] ${testid('http-node-name')}`).click();
        await $(testid('http-response-idle')).waitForDisplayed({ timeout: 10_000 });
        const tab = `${testid('http-tab')}[data-key="${created.id}"]`;

        await $(testid('http-send')).click();
        await $(`${testid('http-response-sending')} app-spinner`).waitForDisplayed({ timeout: 10_000 });
        expect(await $(`${testid('http-send')} app-spinner`).isDisplayed()).toBe(true);
        expect(await $(`${tab} ${testid('http-tab-sending')}`).isDisplayed()).toBe(true);
        const first = await $(testid('http-response-elapsed')).getText();
        await eventually(
          () => $(testid('http-response-elapsed')).getText(),
          (elapsed) => elapsed !== first,
          'the elapsed time to move',
        );

        answer();
        await eventually(
          () => $(testid('http-response-status')).getText(),
          (text) => text === '200 OK',
          'the late answer',
        );
        expect(await $(`${tab} ${testid('http-tab-sending')}`).isExisting()).toBe(false);
        expect(await $(`${testid('http-send')} app-spinner`).isExisting()).toBe(false);

        await $(`${tab} ${testid('http-tab-close')}`).click();
        await $(tab).waitForExist({ reverse: true, timeout: 10_000 });
        await bridge.deleteHttpItem({ kind: 'collection', id: api.id });
      } finally {
        answer();
        server.close();
        server.closeAllConnections();
      }
    });

    it('opens a WebSocket from Rust, sends a message, reads the echo, and closes it', async () => {
      const echo = await echoSocket();
      try {
        const api = await bridge.createHttpCollection('Temps réel');
        const created = await bridge.createHttpRequest({
          ...draft(api.id, null, 'Flux'),
          kind: 'websocket',
          document: { url: `ws://127.0.0.1:${echo.port}/`, description: '' },
        });
        await press('1', ['Control']);
        await $(testid('http-page')).waitForExist({ reverse: true, timeout: 10_000 });
        await press('3', ['Control']);
        await $(testid('http-page')).waitForDisplayed({ timeout: 10_000 });
        if (!(await $(testid('http-rail')).isExisting())) await press('b', ['Control']);
        await $(`${testid('http-node')}[data-id="${created.id}"] ${testid('http-node-name')}`).click();
        await $(testid('http-socket-connect')).waitForDisplayed({ timeout: 10_000 });

        await $(testid('http-socket-connect')).click();
        await $(`${testid('http-socket-phase')}[data-phase="open"]`).waitForDisplayed({ timeout: 10_000 });
        await setField(testid('http-socket-message'), 'hello');
        await $(testid('http-socket-send')).click();
        await eventually(
          () => readEach(testid('http-socket-entry'), 'text'),
          (rows) => rows.some((row) => row.includes('echo: hello')),
          'the echo in the log',
        );

        await $(testid('http-socket-connect')).click();
        await $(`${testid('http-socket-phase')}[data-phase="closed"]`).waitForDisplayed({ timeout: 10_000 });

        const tab = `${testid('http-tab')}[data-key="${created.id}"]`;
        await $(`${tab} ${testid('http-tab-close')}`).click();
        await $(tab).waitForExist({ reverse: true, timeout: 10_000 });
        await bridge.deleteHttpItem({ kind: 'collection', id: api.id });
      } finally {
        echo.close();
      }
    });

    it('keeps the cookies an answer sets, sends them back, and empties the jar after saying how many', async () => {
      const heard: (string | undefined)[] = [];
      const server = createServer((request, response) => {
        heard.push(request.headers.cookie);
        if (request.url === '/login') response.setHeader('Set-Cookie', ['session=abc; Path=/; HttpOnly']);
        response.end('ok');
      });
      await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve));
      const { port } = server.address() as AddressInfo;
      try {
        const api = await bridge.createHttpCollection('Bocal');
        const created = await bridge.createHttpRequest({
          ...draft(api.id, null, 'Login'),
          document: { url: `http://127.0.0.1:${port}/login`, description: '' },
        });
        await press('1', ['Control']);
        await $(testid('http-page')).waitForExist({ reverse: true, timeout: 10_000 });
        await press('3', ['Control']);
        await $(testid('http-page')).waitForDisplayed({ timeout: 10_000 });
        if (!(await $(testid('http-rail')).isExisting())) await press('b', ['Control']);
        await $(`${testid('http-node')}[data-id="${created.id}"] ${testid('http-node-name')}`).click();
        await $(testid('http-response-idle')).waitForDisplayed({ timeout: 10_000 });

        await $(testid('http-send')).click();
        await $(testid('http-response-status')).waitForDisplayed({ timeout: 10_000 });
        await setField(testid('http-url'), `http://127.0.0.1:${port}/me`);
        await $(testid('http-send')).click();
        await eventually(
          () => Promise.resolve(heard),
          (seen) => seen.length === 2,
          'the second send',
        );
        expect(heard).toEqual([undefined, 'session=abc']);

        await $(testid('http-cookies-open')).click();
        await eventually(
          () =>
            readEach(
              `${testid('http-cookies-domain')}[data-domain="127.0.0.1"] ${testid('http-cookie')}`,
              'text',
            ),
          (rows) => rows.length === 1 && rows[0]!.includes('session'),
          'the cookie in the jar',
        );
        await $(testid('http-cookies-clear')).click();
        await $(testid('http-cookies-clear-count')).waitForDisplayed({ timeout: 10_000 });
        await $(testid('http-cookies-clear-confirm')).click();
        await $(testid('http-cookies-empty')).waitForDisplayed({ timeout: 10_000 });
        await $(testid('http-cookies-close')).click();
        await $(testid('http-cookies-dialog')).waitForExist({ reverse: true, timeout: 10_000 });

        const tab = `${testid('http-tab')}[data-key="${created.id}"]`;
        await $(`${tab} ${testid('http-tab-close')}`).click();
        await $(testid('http-close-discard')).click();
        await $(tab).waitForExist({ reverse: true, timeout: 10_000 });
        await bridge.deleteHttpItem({ kind: 'collection', id: api.id });
      } finally {
        server.close();
      }
    });
  });
});
