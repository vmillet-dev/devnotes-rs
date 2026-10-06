import { $, $$, expect } from '@wdio/globals';

import type { HttpNode, HttpRequestDraft, HttpTree } from '@core/ipc/bindings';

import { canvas } from '../pageobjects/canvas.page.js';
import { eventually, press, readEach, setField, testid } from '../support/app.js';
import { bridge } from '../support/bridge.js';

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
    expect(outline(await bridge.httpTree()).some((line) => line.startsWith('API Paiements'))).toBe(false);
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
        () => $(`${testid('http-open-request')} h1`).getText(),
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
});
