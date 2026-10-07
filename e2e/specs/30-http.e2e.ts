import { expect } from '@wdio/globals';

import type { HttpNode, HttpRequestDraft, HttpTree } from '@core/ipc/bindings';

import { canvas } from '../pageobjects/canvas.page.js';
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
});
