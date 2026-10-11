import { createServer } from 'node:http';
import type { AddressInfo } from 'node:net';

import { $, $$, browser, expect } from '@wdio/globals';

import type { HttpNode, HttpRequestDraft, HttpTree } from '@core/ipc/bindings';

import { canvas } from '../pageobjects/canvas.page.js';
import {
  eventually,
  press,
  readEach,
  reopenSession,
  setField,
  testid,
  viewportSize,
} from '../support/app.js';
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

    /**
     * The rail's floor is 160px, and HTTP took 240 because its rows did not fit. Made compact —
     * a narrower indent, no room held for a caret or for `⋯` at rest, a short method — a
     * request two folders deep keeps a dozen characters of its name.
     */
    it('shows a request two folders deep at the floor of the rail, a dozen characters of its name', async () => {
      const api = await bridge.createHttpCollection('Étroit');
      const outer = await bridge.createHttpFolder(api.id, null, 'Commandes');
      const inner = await bridge.createHttpFolder(api.id, outer.id, 'Archives');
      const deep = await bridge.createHttpRequest({
        ...draft(api.id, inner.id, 'Supprimer la commande archivée'),
        method: 'DELETE',
      });
      await press('1', ['Control']);
      await $(testid('http-page')).waitForExist({ reverse: true, timeout: 10_000 });
      await press('3', ['Control']);
      await $(testid('http-rail')).waitForDisplayed({ timeout: 10_000 });
      await browser.execute(
        (selector: string) => (document.querySelector(selector) as HTMLElement | null)?.focus(),
        testid('http-rail-edge'),
      );
      await browser.keys(Array.from({ length: 30 }, () => 'ArrowLeft'));
      const row = `${testid('http-node')}[data-id="${deep.id}"]`;
      await $(row).waitForDisplayed({ timeout: 10_000 });

      const measured = await eventually(
        () =>
          browser.execute(
            (rail: string, target: string) => {
              const name = document.querySelector<HTMLElement>(`${target} [data-testid="http-node-name"]`)!;
              const label = name.querySelector<HTMLElement>('.node-label')!;
              const style = getComputedStyle(label);
              // A dozen characters of this very font: what the label has to hold.
              const ruler = document.createElement('canvas').getContext('2d')!;
              ruler.font = `${style.fontWeight} ${style.fontSize} ${style.fontFamily}`;
              return {
                rail: Math.round(document.querySelector(rail)!.getBoundingClientRect().width),
                label: label.getBoundingClientRect().width,
                dozen: ruler.measureText('Supprimer la').width,
                badge: name.querySelector('[data-testid="http-node-badge"]')?.textContent?.trim(),
                title: name.getAttribute('title'),
              };
            },
            testid('http-rail'),
            row,
          ),
        (seen) => seen.rail === 160,
        'the rail at its floor',
      );

      expect(measured.badge).toBe('DEL');
      expect(measured.title).toBe('Supprimer la commande archivée');
      expect(measured.label).toBeGreaterThanOrEqual(measured.dozen);
      await bridge.deleteHttpItem({ kind: 'collection', id: api.id });
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

    /**
     * A request's name lived in two places: renamed from the rail, its open tab and its title
     * kept the old one, and the next save wrote it back over the new one.
     */
    it('renames an open request from the rail or from its tab, and keeps one name everywhere', async () => {
      const api = await bridge.createHttpCollection('Renommage');
      const created = await bridge.createHttpRequest(draft(api.id, null, 'Avant'));
      await press('1', ['Control']);
      await $(testid('http-page')).waitForExist({ reverse: true, timeout: 10_000 });
      await press('3', ['Control']);
      await $(testid('http-page')).waitForDisplayed({ timeout: 10_000 });
      if (!(await $(testid('http-rail')).isExisting())) await press('b', ['Control']);
      const row = `${testid('http-node')}[data-id="${created.id}"]`;
      const tab = `${testid('http-tab')}[data-key="${created.id}"]`;
      /** The three places a request's name is written, and the one Rust holds. */
      const names = async () => ({
        row: (await $(`${row} ${testid('http-node-name')}`).getText()).replace(/^GET\s*/, ''),
        title: await $(testid('http-request-name')).getValue(),
        tab: (await $(`${tab} ${testid('http-tab-open')}`).getText()).includes(
          await $(testid('http-request-name')).getValue(),
        ),
        kept: (await bridge.httpRequest(created.id)).name,
      });
      const everywhere = (name: string) => ({ row: name, title: name, tab: true, kept: name });

      await $(`${row} ${testid('http-node-name')}`).click();
      await eventually(names, (seen) => seen.title === 'Avant', 'the request in its tab');

      await choose(await $(row).getElement(), 'rename');
      await setField(testid('http-rename-input'), 'Après');
      await $(testid('http-rename-submit')).click();
      expect(
        await eventually(names, (seen) => seen.title === 'Après', 'the title to follow the rail'),
      ).toEqual(everywhere('Après'));
      expect(await $(`${tab} ${testid('http-tab-dirty')}`).isExisting()).toBe(false);

      // The save that used to write « Avant » back: something else modified, then Ctrl+S.
      await setField(testid('http-url'), 'https://api.exemple.fr/apres');
      await press('s', ['Control']);
      await $(`${tab} ${testid('http-tab-dirty')}`).waitForExist({ reverse: true, timeout: 10_000 });
      expect(await names()).toEqual(everywhere('Après'));

      // The other way: a name typed in the title reaches the rail once saved.
      await setField(testid('http-request-name'), 'Encore');
      await press('s', ['Control']);
      expect(await eventually(names, (seen) => seen.row === 'Encore', 'the rail to follow the tab')).toEqual(
        everywhere('Encore'),
      );

      await press('1', ['Control']);
      await $(testid('http-page')).waitForExist({ reverse: true, timeout: 10_000 });
      await press('3', ['Control']);
      await $(row).waitForDisplayed({ timeout: 10_000 });
      expect(await names()).toEqual(everywhere('Encore'));

      await reopenSession();
      await $(testid('http-page')).waitForDisplayed({ timeout: 10_000 });
      await $(row).waitForDisplayed({ timeout: 10_000 });
      expect(await eventually(names, (seen) => seen.title === 'Encore', 'the tab restored')).toEqual(
        everywhere('Encore'),
      );

      await $(`${tab} ${testid('http-tab-close')}`).click();
      await $(tab).waitForExist({ reverse: true, timeout: 10_000 });
      await bridge.deleteHttpItem({ kind: 'collection', id: api.id });
    });

    /**
     * Key `X-Recette: 1`, value `rsgfdregr`, in a folder's settings: accepted as typed, and
     * every request under the folder then failed « Cette URL ne peut pas être envoyée », on a
     * good URL.
     */
    it('splits a header typed with its value, refuses a name that cannot be sent, and still sends under the folder', async () => {
      const heard: (string | undefined)[] = [];
      const server = createServer((request, response) => {
        heard.push(request.headers['x-recette']?.toString());
        response.writeHead(200, { 'Content-Type': 'text/plain' });
        response.end('ok');
      });
      await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve));
      const { port } = server.address() as AddressInfo;
      try {
        const api = await bridge.createHttpCollection('Recette');
        const folder = await bridge.createHttpFolder(api.id, null, 'Sous test');
        const created = await bridge.createHttpRequest({
          ...draft(api.id, folder.id, 'Ping'),
          document: { url: `http://127.0.0.1:${port}/ping`, description: '' },
        });
        await press('1', ['Control']);
        await $(testid('http-page')).waitForExist({ reverse: true, timeout: 10_000 });
        await press('3', ['Control']);
        await $(testid('http-page')).waitForDisplayed({ timeout: 10_000 });
        if (!(await $(testid('http-rail')).isExisting())) await press('b', ['Control']);
        const kept = async () => (await bridge.httpSettings({ kind: 'folder', id: folder.id })).headers ?? [];
        const row = `${testid('http-settings-headers-table')} ${testid('http-kv-row')}:first-child`;
        const name = `${row} ${testid('http-kv-key')}`;
        const cells = async () => [
          await $(name).getValue(),
          (await $(`${row} ${testid('http-kv-value')}`).getValue()).trim(),
        ];

        const node = `${testid('http-node')}[data-id="${folder.id}"]`;
        await $(node).waitForDisplayed({ timeout: 10_000 });
        await choose(await $(node).getElement(), 'settings');
        await $(testid('http-settings-dialog')).waitForDisplayed({ timeout: 10_000 });

        // Key by key, as it is typed: at the colon the caret goes on to the value.
        await $(name).click();
        await browser.keys([...'X-Recette: 1']);
        await eventually(cells, (read) => read.join('|') === 'X-Recette|1', 'the header in its two cells');
        expect(await $(`${row}[data-fault]`).isExisting()).toBe(false);

        // A name that no split mends: marked, said, and refused when it is kept.
        await setField(name, 'X Recette');
        await $(`${row}[data-fault="name"]`).waitForExist({ timeout: 10_000 });
        await $(`${testid('http-kv-fault')}[data-fault="name"]`).waitForDisplayed({ timeout: 10_000 });
        await $(testid('http-settings-save')).click();
        await eventually(
          () => $(testid('error-banner')).getText(),
          (said) => said.includes('X Recette'),
          'the refusal, naming the header',
        );
        expect(await $(testid('http-settings-dialog')).isDisplayed()).toBe(true);
        expect(await kept()).toEqual([]);
        await $(testid('error-dismiss')).click();

        await setField(name, 'X-Recette');
        await $(`${row}[data-fault]`).waitForExist({ reverse: true, timeout: 10_000 });
        await $(testid('http-settings-save')).click();
        await $(testid('http-settings-dialog')).waitForExist({ reverse: true, timeout: 10_000 });
        expect((await kept()).map((header) => header.key)).toEqual(['X-Recette']);

        // The request under the folder leaves, the header with it.
        await $(`${testid('http-node')}[data-id="${created.id}"] ${testid('http-node-name')}`).click();
        await $(testid('http-response-idle')).waitForDisplayed({ timeout: 10_000 });
        await $(testid('http-send')).click();
        await eventually(
          async () => heard.length,
          (reached) => reached === 1,
          'the request under the folder to reach the server',
        );
        expect(heard).toEqual(['1']);
        await $(testid('http-response-sending')).waitForExist({ reverse: true, timeout: 10_000 });

        const tab = `${testid('http-tab')}[data-key="${created.id}"]`;
        await $(`${tab} ${testid('http-tab-close')}`).click();
        await $(tab).waitForExist({ reverse: true, timeout: 10_000 });
        await bridge.deleteHttpItem({ kind: 'collection', id: api.id });
      } finally {
        server.close();
      }
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
    /**
     * A `POST` with a JSON body, switched to `GET`: the body stayed in its tab and was sent —
     * `Content-Length: 24` on a `GET` — and nothing on screen said so.
     */
    it('says a GET carries a body, on its tab and above it, and sends it all the same', async () => {
      const heard: { method: string | undefined; type: string | undefined; body: string }[] = [];
      const server = createServer((request, response) => {
        let body = '';
        request.on('data', (chunk: Buffer) => (body += chunk.toString()));
        request.on('end', () => {
          heard.push({ method: request.method, type: request.headers['content-type'], body });
          response.writeHead(200, { 'Content-Type': 'text/plain' });
          response.end('ok');
        });
      });
      await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve));
      const { port } = server.address() as AddressInfo;
      try {
        const text = '{"q":"devnotes"}';
        const api = await bridge.createHttpCollection('Recherche');
        const created = await bridge.createHttpRequest({
          ...draft(api.id, null, 'Chercher'),
          method: 'POST',
          document: { url: `http://127.0.0.1:${port}/search`, description: '', body: { kind: 'json', text } },
        });
        await press('1', ['Control']);
        await $(testid('http-page')).waitForExist({ reverse: true, timeout: 10_000 });
        await press('3', ['Control']);
        await $(testid('http-page')).waitForDisplayed({ timeout: 10_000 });
        if (!(await $(testid('http-rail')).isExisting())) await press('b', ['Control']);
        await $(`${testid('http-node')}[data-id="${created.id}"] ${testid('http-node-name')}`).click();
        await $(testid('http-response-idle')).waitForDisplayed({ timeout: 10_000 });
        const mark = testid('http-body-unusual-mark');
        const method = async (id: string) => {
          await $(testid('http-method')).click();
          await $(
            `${testid('choice-panel-http-method')} ${testid('choice-option')}[data-option-id="${id}"]`,
          ).click();
        };

        // Seen from « Paramètres », the tab in front: a POST's body is marked by nothing.
        await $(`${testid('http-section')}[data-section="params"]`).click();
        expect(await $(mark).isExisting()).toBe(false);
        await method('GET');
        await $(mark).waitForDisplayed({ timeout: 10_000 });
        expect(await $(testid('http-body-unusual')).isExisting()).toBe(false);

        await $(`${testid('http-section')}[data-section="body"]`).click();
        await $(testid('http-body-unusual')).waitForDisplayed({ timeout: 10_000 });

        // Sent as typed all the same: some APIs do read it.
        await $(testid('http-send')).click();
        await eventually(
          async () => heard.length,
          (reached) => reached === 1,
          'the GET to reach the server',
        );
        expect(heard[0]).toEqual({ method: 'GET', type: 'application/json', body: text });
        await $(testid('http-response-sending')).waitForExist({ reverse: true, timeout: 10_000 });

        // « Passer à Aucun » sends none and empties nothing: the text is back under its type.
        await $(testid('http-body-to-none')).click();
        await $(mark).waitForExist({ reverse: true, timeout: 10_000 });
        expect(await $(testid('http-body')).getAttribute('data-kind')).toBe('none');
        expect(await $(testid('http-body-unusual')).isExisting()).toBe(false);
        await $(testid('http-send')).click();
        await eventually(
          async () => heard.length,
          (reached) => reached === 2,
          'the bare GET to reach the server',
        );
        expect(heard[1]).toEqual({ method: 'GET', type: undefined, body: '' });
        await $(testid('http-response-sending')).waitForExist({ reverse: true, timeout: 10_000 });

        await $(`${testid('segmented-http-body')} [data-segment-id="json"]`).click();
        await eventually(
          () => $(testid('http-body-text')).getValue(),
          (kept) => kept === text,
          'the text back as it was',
        );
        await $(mark).waitForDisplayed({ timeout: 10_000 });

        await method('POST');
        await $(mark).waitForExist({ reverse: true, timeout: 10_000 });
        expect(await $(testid('http-body-unusual')).isExisting()).toBe(false);

        const tab = `${testid('http-tab')}[data-key="${created.id}"]`;
        await $(`${tab} ${testid('http-tab-close')}`).click();
        if (await $(testid('http-close-discard')).isExisting()) await $(testid('http-close-discard')).click();
        await $(tab).waitForExist({ reverse: true, timeout: 10_000 });
        await bridge.deleteHttpItem({ kind: 'collection', id: api.id });
      } finally {
        server.close();
      }
    });

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
      // Its own history: the scenarios before it send too, and it counts what it sent.
      await bridge.clearHttpHistory();
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
     * `api.github.com` answered « Please make sure your request has a User-Agent header »: none
     * was sent, and nothing in the editor said what leaves without having been typed.
     */
    it('names the application to the server unless a row says otherwise, and lists what leaves unasked', async () => {
      const agents: (string | undefined)[] = [];
      const server = createServer((request, response) => {
        agents.push(request.headers['user-agent']);
        response.writeHead(200, { 'Content-Type': 'text/plain' });
        response.end('ok');
      });
      await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve));
      const { port } = server.address() as AddressInfo;
      try {
        const api = await bridge.createHttpCollection('Agent');
        const created = await bridge.createHttpRequest({
          ...draft(api.id, null, 'Qui'),
          document: { url: `http://127.0.0.1:${port}/who`, description: '' },
        });
        await press('1', ['Control']);
        await $(testid('http-page')).waitForExist({ reverse: true, timeout: 10_000 });
        await press('3', ['Control']);
        await $(testid('http-page')).waitForDisplayed({ timeout: 10_000 });
        if (!(await $(testid('http-rail')).isExisting())) await press('b', ['Control']);
        await $(`${testid('http-node')}[data-id="${created.id}"] ${testid('http-node-name')}`).click();
        await $(testid('http-response-idle')).waitForDisplayed({ timeout: 10_000 });
        await $(`${testid('http-section')}[data-section="headers"]`).click();

        /** The list under the table, in its order: « name (who put it there) » and its value. */
        const implied = () =>
          browser.execute(
            (row: string, value: string) =>
              [...document.querySelectorAll<HTMLElement>(row)].map((item) => [
                `${item.dataset['name']} (${item.dataset['source']})`,
                item.querySelector(value)?.textContent ?? '',
              ]),
            testid('http-implied-header'),
            testid('http-implied-value'),
          );
        const names = async () => (await implied()).map(([name]) => name);
        /** Sends, and answers the agent the server read once the answer is shown. */
        const sent = async (count: number) => {
          await $(testid('http-send')).click();
          await eventually(
            async () => agents.length,
            (reached) => reached === count,
            `send ${count} to reach the server`,
          );
          await $(testid('http-response-sending')).waitForExist({ reverse: true, timeout: 10_000 });
          return agents[count - 1];
        };
        const firstRow = `${testid('http-headers-table')} ${testid('http-kv-row')}:first-child`;

        const listed = await eventually(implied, (rows) => rows.length === 3, 'what leaves unasked, listed');
        const agent = listed[0]![1]!;
        expect(listed).toEqual([
          ['User-Agent (default)', expect.stringMatching(/^DevNotes\/\d+\.\d+\.\d+$/)],
          ['Accept (default)', '*/*'],
          ['Host (url)', `127.0.0.1:${port}`],
        ]);

        // What the server read is what the list said, and the timeline says it too.
        expect(await sent(1)).toBe(agent);
        await $(`${testid('http-response-section')}[data-section="timeline"]`).click();
        expect(await $(testid('http-response-sent')).getText()).toContain(`User-Agent: ${agent}`);

        // A row of the request's own replaces it, and leaves the list.
        await setField(`${firstRow} ${testid('http-kv-key')}`, 'User-Agent');
        await setField(`${firstRow} ${testid('http-kv-value')}`, 'curl/8.5.0');
        await eventually(
          names,
          (seen) => seen.join() === 'Accept (default),Host (url)',
          'the default off the list',
        );
        expect(await sent(2)).toBe('curl/8.5.0');

        // The same row switched off: none at all, and none listed.
        await $(`${firstRow} ${testid('http-kv-enabled')}`).click();
        expect(await sent(3)).toBeUndefined();
        expect(await names()).toEqual(['Accept (default)', 'Host (url)']);

        // Handed down by the collection, once the request's own row is gone: listed as inherited.
        await bridge.saveHttpSettings(
          { kind: 'collection', id: api.id },
          {
            auth: { kind: 'inherit' },
            headers: [{ enabled: true, key: 'User-Agent', value: 'Equipe/1', description: '' }],
            transport: {},
          },
        );
        await $(`${firstRow} ${testid('http-kv-remove')}`).click();
        expect(
          await eventually(
            implied,
            (rows) => rows[0]?.[0] === 'User-Agent (above)',
            'the inherited one, listed',
          ),
        ).toEqual([
          ['User-Agent (above)', 'Equipe/1'],
          ['Accept (default)', '*/*'],
          ['Host (url)', `127.0.0.1:${port}`],
        ]);
        expect(await sent(4)).toBe('Equipe/1');

        const tab = `${testid('http-tab')}[data-key="${created.id}"]`;
        await $(`${tab} ${testid('http-tab-close')}`).click();
        if (await $(testid('http-close-discard')).isExisting()) await $(testid('http-close-discard')).click();
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
        // The handshake is composed like a request: it names the application too.
        expect(echo.handshakes.map((headers) => headers['user-agent'])).toEqual([
          expect.stringMatching(/^DevNotes\/\d+\.\d+\.\d+$/),
        ]);
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

    /**
     * `/cookies/set?session=abc123` answers a 302 that sets the cookie and leads to a page
     * that echoes what it received: it received none, « Cookies » read « La réponse ne pose
     * aucun cookie », and the jar stayed empty. The shape of nearly every login.
     */
    it('sends a cookie set on a redirect to the page it leads to, lists and keeps it, unless cookies are off', async () => {
      const server = createServer((request, response) => {
        if (request.url?.startsWith('/cookies/set')) {
          response.writeHead(302, { Location: '/cookies', 'Set-Cookie': 'session=abc123; Path=/' });
          response.end();
          return;
        }
        response.writeHead(200, { 'Content-Type': 'application/json' });
        response.end(JSON.stringify({ cookies: request.headers.cookie ?? null }));
      });
      await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve));
      const { port } = server.address() as AddressInfo;
      try {
        await bridge.clearHttpCookies();
        const api = await bridge.createHttpCollection('Connexion');
        const created = await bridge.createHttpRequest({
          ...draft(api.id, null, 'Se connecter'),
          document: { url: `http://127.0.0.1:${port}/cookies/set?session=abc123`, description: '' },
        });
        await press('1', ['Control']);
        await $(testid('http-page')).waitForExist({ reverse: true, timeout: 10_000 });
        await press('3', ['Control']);
        await $(testid('http-page')).waitForDisplayed({ timeout: 10_000 });
        if (!(await $(testid('http-rail')).isExisting())) await press('b', ['Control']);
        await $(`${testid('http-node')}[data-id="${created.id}"] ${testid('http-node-name')}`).click();
        await $(testid('http-response-idle')).waitForDisplayed({ timeout: 10_000 });
        /** Sends, and answers what the page the redirect leads to says it received. */
        const echoed = async () => {
          await $(testid('http-send')).click();
          await $(testid('http-response-sending')).waitForExist({ reverse: true, timeout: 10_000 });
          await $(`${testid('http-response-section')}[data-section="body"]`).click();
          await $(`${testid('segmented-http-response-view')} [data-segment-id="raw"]`).click();
          return eventually(
            () => $(testid('http-response-body')).getText(),
            (body) => body.startsWith('{"cookies":'),
            'the echo of the page the redirect leads to',
          );
        };
        const jarRows = () =>
          readEach(
            `${testid('http-cookies-domain')}[data-domain="127.0.0.1"] ${testid('http-cookie')}`,
            'text',
          );

        expect(await echoed()).toBe('{"cookies":"session=abc123"}');

        // Set by the redirect, not by the answer: the tab says which, the timeline shows it there.
        await $(`${testid('http-response-section')}[data-section="cookies"]`).click();
        expect(
          await eventually(
            () =>
              browser.execute(
                (row: string) =>
                  [...document.querySelectorAll<HTMLElement>(row)].map((cookie) => [
                    cookie.querySelector('.key')?.textContent?.trim(),
                    cookie.dataset['setBy'],
                  ]),
                testid('http-response-cookie'),
              ),
            (rows) => rows.length > 0,
            'the cookie of the redirect, listed',
          ),
        ).toEqual([['session', 'redirect']]);
        await $(`${testid('http-response-section')}[data-section="timeline"]`).click();
        expect(await readEach(testid('http-response-redirect-cookie'), 'text')).toEqual([
          'Set-Cookie: session=abc123',
        ]);

        await $(testid('http-cookies-open')).click();
        await eventually(
          jarRows,
          (rows) => rows.length === 1 && rows[0]!.includes('session'),
          'the cookie in the jar',
        );
        await $(testid('http-cookies-close')).click();
        await $(testid('http-cookies-dialog')).waitForExist({ reverse: true, timeout: 10_000 });

        // « Envoyer et garder les cookies : Non »: none at any hop, none kept.
        await bridge.clearHttpCookies();
        await $(`${testid('http-section')}[data-section="settings"]`).click();
        await $(testid('http-transport-useCookies')).click();
        await $(`${testid('choice-option')}[data-option-id="off"]`).click();
        expect(await echoed()).toBe('{"cookies":null}');
        await $(testid('http-cookies-open')).click();
        await $(testid('http-cookies-empty')).waitForDisplayed({ timeout: 10_000 });
        await $(testid('http-cookies-close')).click();
        await $(testid('http-cookies-dialog')).waitForExist({ reverse: true, timeout: 10_000 });

        const tab = `${testid('http-tab')}[data-key="${created.id}"]`;
        await $(`${tab} ${testid('http-tab-close')}`).click();
        await $(testid('http-close-discard')).click();
        await $(tab).waitForExist({ reverse: true, timeout: 10_000 });
        await bridge.deleteHttpItem({ kind: 'collection', id: api.id });
      } finally {
        await bridge.clearHttpCookies();
        server.close();
      }
    });
  });
});
