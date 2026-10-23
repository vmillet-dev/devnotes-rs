import { createServer } from 'node:http';
import type { AddressInfo } from 'node:net';

import { $, $$, browser, expect } from '@wdio/globals';

import type { HttpNode, HttpRequestDraft, HttpTree } from '@core/ipc/bindings';

import { canvas } from '../pageobjects/canvas.page.js';
import {
  blurField,
  eventually,
  hover,
  preferenceWritten,
  press,
  readEach,
  reloadCanvas,
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
    /** The three kinds of request are one entry of a row's menu, and a submenu beside it. */
    const choose = async (row: WebdriverIO.Element, action: string) => {
      await row.$(testid('http-node-menu')).click();
      if (/^new(Http|Graphql|Websocket)$/.test(action)) await $(testid('http-node-new')).click();
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

    /**
     * GraphQL and WebSocket shipped and were not found: the last two entries of the method
     * menu, a kind filed among verbs. A request is begun as what it is.
     */
    it('begins a request as HTTP, GraphQL or a WebSocket, from the tabs and from a collection', async () => {
      const api = await bridge.createHttpCollection('Trois types');
      await press('1', ['Control']);
      await $(testid('http-page')).waitForExist({ reverse: true, timeout: 10_000 });
      await press('3', ['Control']);
      await $(testid('http-page')).waitForDisplayed({ timeout: 10_000 });
      if (!(await $(testid('http-rail')).isExisting())) await press('b', ['Control']);
      const kinds = () => readEach(testid('http-tab'), '@data-kind');
      const tabCount = async () => (await $$(testid('http-tab')).getElements()).length;
      const before = await tabCount();

      // « Nouvelle requête », at the end of the tabs: an HTTP request in one click, no menu.
      await $(testid('http-tab-new')).click();
      await eventually(tabCount, (count) => count === before + 1, 'the HTTP request begun');
      expect(await $(testid('http-new-kind-panel')).isExisting()).toBe(false);
      await $(testid('http-method')).waitForDisplayed({ timeout: 10_000 });
      expect(await $(testid('http-kind-badge')).isExisting()).toBe(false);

      // Its chevron: the three kinds, the first in focus, the panel whole in the window.
      await $(testid('http-tab-new-kind')).click();
      expect(await readEach(testid('http-new-kind'), '@data-kind')).toEqual(['http', 'graphql', 'websocket']);
      await eventually(
        () => browser.execute(() => document.activeElement?.getAttribute('data-kind') ?? null),
        (kind) => kind === 'http',
        'the HTTP entry in focus',
      );
      expect(
        await browser.execute((panel: string) => {
          const box = document.querySelector(panel)!.getBoundingClientRect();
          return box.left >= 0 && box.right <= window.innerWidth && box.bottom <= window.innerHeight;
        }, testid('http-new-kind-panel')),
      ).toBe(true);
      // Above the request it opens over, edge to edge: the URL field once covered its badges.
      expect(
        await browser.execute(
          (panel: string, entry: string) => {
            const menu = document.querySelector(panel)!;
            return [...menu.querySelectorAll(entry)].every((option) => {
              const box = option.getBoundingClientRect();
              return [0.05, 0.5, 0.95].every((at) =>
                menu.contains(document.elementFromPoint(box.left + box.width * at, box.top + box.height / 2)),
              );
            });
          },
          testid('http-new-kind-panel'),
          testid('http-new-kind'),
        ),
      ).toBe(true);
      await $(`${testid('http-new-kind')}[data-kind="graphql"]`).click();
      await $(testid('http-graphql')).waitForDisplayed({ timeout: 10_000 });
      // Its kind is said where a method would be chosen, and there is no menu to change it.
      expect(await $(testid('http-kind-badge')).getAttribute('data-kind')).toBe('graphql');
      expect(await $(testid('http-method')).isExisting()).toBe(false);

      await $(testid('http-tab-new-kind')).click();
      await $(`${testid('http-new-kind')}[data-kind="websocket"]`).click();
      await $(testid('http-socket-connect')).waitForDisplayed({ timeout: 10_000 });
      expect(await $(testid('http-kind-badge')).getAttribute('data-kind')).toBe('websocket');
      expect((await kinds()).slice(-3)).toEqual(['http', 'graphql', 'websocket']);

      // From a collection's menu: the kind first, then the name; Rust holds it as that kind.
      const collection = `${testid('http-node')}[data-id="${api.id}"]`;
      await $(collection).waitForDisplayed({ timeout: 10_000 });
      // On Linux the rail once ran 6px wider than itself: the click scrolled it, and the scroll
      // closed the menu it had just opened.
      expect(
        await browser.execute((rail: string) => {
          const box = document.querySelector<HTMLElement>(rail)!;
          return box.scrollWidth - box.clientWidth;
        }, testid('http-rail')),
      ).toBe(0);
      // Its menu opens under its `⋯`, whole: it once opened a row's width away, cut by the rail.
      await $(`${collection} ${testid('http-node-menu')}`).click();
      await $(testid('http-node-menu-panel')).waitForDisplayed({ timeout: 10_000 });
      expect(
        await browser.execute(
          (trigger: string, panel: string) => {
            const from = document.querySelector(trigger)!.getBoundingClientRect();
            const box = document.querySelector(panel)!.getBoundingClientRect();
            const under = box.top - from.bottom;
            return {
              atTrigger: Math.abs(box.left - from.left) <= 1 && under >= 0 && under <= 12,
              whole: box.width >= 200 && box.right <= window.innerWidth && box.bottom <= window.innerHeight,
            };
          },
          `${collection} ${testid('http-node-menu')}`,
          testid('http-node-menu-panel'),
        ),
      ).toEqual({ atTrigger: true, whole: true });
      // The three kinds of request are one entry of it — listed flat they made eight — and
      // a submenu beside the panel, on the workspace's side.
      expect(
        await readEach(`${testid('http-node-menu-panel')} ${testid('http-node-action')}`, '@data-action'),
      ).toEqual(['newFolder', 'run', 'settings', 'rename', 'duplicate', 'delete']);
      // The pointer resting on its entry opens it, with no click.
      await hover(testid('http-node-new'));
      await $(testid('http-node-new-menu')).waitForDisplayed({ timeout: 10_000 });
      expect(
        await readEach(`${testid('http-node-new-menu')} ${testid('http-node-action')}`, '@data-action'),
      ).toEqual(['newHttp', 'newGraphql', 'newWebsocket']);
      expect(
        await browser.execute(
          (panel: string, submenu: string) => {
            const beside = document.querySelector(panel)!.getBoundingClientRect();
            const box = document.querySelector(submenu)!.getBoundingClientRect();
            return (
              box.left >= beside.right && box.right <= window.innerWidth && box.bottom <= window.innerHeight
            );
          },
          testid('http-node-menu-panel'),
          testid('http-node-new-menu'),
        ),
      ).toBe(true);
      // Another entry hovered folds it, and the menu stays.
      await hover(`${testid('http-node-menu-panel')} ${testid('http-node-action')}[data-action="rename"]`);
      await $(testid('http-node-new-menu')).waitForExist({ reverse: true, timeout: 10_000 });
      expect(await $(testid('http-node-menu-panel')).isExisting()).toBe(true);
      // A click opens it too; Escape folds the submenu first, the menu next.
      await $(testid('http-node-new')).click();
      await $(testid('http-node-new-menu')).waitForDisplayed({ timeout: 10_000 });
      await press('Escape');
      await $(testid('http-node-new-menu')).waitForExist({ reverse: true, timeout: 10_000 });
      expect(await $(testid('http-node-menu-panel')).isExisting()).toBe(true);
      await press('Escape');
      await $(testid('http-node-menu-panel')).waitForExist({ reverse: true, timeout: 10_000 });
      await choose(await $(collection).getElement(), 'newGraphql');
      await create('Produits');
      await eventually(
        rowNames,
        (names) => names.some((name) => name.endsWith('Produits')),
        'the GraphQL request in the rail',
      );
      await choose(await $(collection).getElement(), 'newWebsocket');
      await create('Flux');
      await eventually(
        rowNames,
        (names) => names.some((name) => name.endsWith('Flux')),
        'the socket in the rail',
      );
      const held = (await bridge.httpTree()).collections
        .find((node) => node.collection.id === api.id)!
        .children.map((node) => (node.kind === 'request' ? [node.request.name, node.request.kind] : []));
      expect(held).toEqual([
        ['Produits', 'graphql'],
        ['Flux', 'websocket'],
      ]);

      // The three drafts and the two requests the rail opened are closed, nothing kept of them.
      for (let left = await tabCount(); left > before; left = await tabCount()) {
        await $(`${testid('http-tab')}:last-of-type ${testid('http-tab-close')}`).click();
        if (await $(testid('http-close-discard')).isExisting()) await $(testid('http-close-discard')).click();
        await eventually(tabCount, (count) => count === left - 1, 'a tab closed');
      }
      await bridge.deleteHttpItem({ kind: 'collection', id: api.id });
    });

    /**
     * Two « + » a few pixels apart, one glyph for two jobs, and a name field that stayed open,
     * unfocused, once left by a click: the rail's first steps hesitated.
     */
    it('offers the first collection from the empty rail, and keeps a name whose field is left', async () => {
      // A library with no collection: what the scenarios before this one left is taken out.
      for (const node of (await bridge.httpTree()).collections) {
        await bridge.deleteHttpItem({ kind: 'collection', id: node.collection.id });
      }
      await press('1', ['Control']);
      await $(testid('http-page')).waitForExist({ reverse: true, timeout: 10_000 });
      await press('3', ['Control']);
      await $(testid('http-page')).waitForDisplayed({ timeout: 10_000 });
      if (!(await $(testid('http-rail')).isExisting())) await press('b', ['Control']);
      const held = async () => outline(await bridge.httpTree());
      const edge = async (key: 'ArrowLeft' | 'ArrowRight', times: number) => {
        await browser.execute(
          (selector: string) => (document.querySelector(selector) as HTMLElement | null)?.focus(),
          testid('http-rail-edge'),
        );
        await browser.keys(Array.from({ length: times }, () => key));
      };
      const worded = () => $(testid('http-collection-create-open')).getAttribute('data-worded');

      // The head's button is not the tabs' « + »: an icon at the floor, its name once it fits.
      await edge('ArrowLeft', 30);
      await eventually(worded, (seen) => seen === 'false', 'the icon at the floor of the rail');
      await edge('ArrowRight', 4);
      await eventually(worded, (seen) => seen === 'true', 'the button named on a wider rail');
      expect(
        await browser.execute(
          (button: string, rail: string) =>
            document.querySelector(button)!.getBoundingClientRect().right <=
            document.querySelector(rail)!.getBoundingClientRect().right,
          testid('http-collection-create-open'),
          testid('http-rail'),
        ),
      ).toBe(true);
      await edge('ArrowLeft', 30);

      // The empty rail offers the action itself, and not a pointer to a glyph.
      await $(testid('http-rail-empty-create')).click();
      await create('Premiers pas');
      await eventually(rowNames, (names) => names.includes('Premiers pas'), 'the first collection');
      expect(await $(testid('http-rail-empty')).isExisting()).toBe(false);

      // A name typed, then the field left for the rest of the window: created, once.
      await $(testid('http-collection-create-open')).click();
      await setField(testid('http-create-input'), 'Laissée');
      await blurField();
      await eventually(rowNames, (names) => names.includes('Laissée'), 'the collection whose field was left');
      await $(testid('http-create-input')).waitForExist({ reverse: true, timeout: 10_000 });
      expect(await held()).toEqual(['Premiers pas[]', 'Laissée[]']);

      // A rename left the same way is written, and still there once the front end is rebuilt.
      await choose((await named('Laissée'))!, 'rename');
      await setField(testid('http-rename-input'), 'Gardée');
      await blurField();
      await eventually(rowNames, (names) => names.includes('Gardée'), 'the rename kept');
      await $(testid('http-rename-input')).waitForExist({ reverse: true, timeout: 10_000 });
      await reloadCanvas();
      await $(testid('http-page')).waitForDisplayed({ timeout: 10_000 });
      await eventually(rowNames, (names) => names.includes('Gardée'), 'the rename after a reload');
      expect(await held()).toEqual(['Premiers pas[]', 'Gardée[]']);

      // Escape is the way to cancel, whatever was typed: the tree is as it was.
      await $(testid('http-collection-create-open')).click();
      await setField(testid('http-create-input'), 'Jamais');
      await press('Escape');
      await $(testid('http-create-input')).waitForExist({ reverse: true, timeout: 10_000 });
      await choose((await named('Gardée'))!, 'rename');
      await setField(testid('http-rename-input'), 'Jamais');
      await press('Escape');
      await $(testid('http-rename-input')).waitForExist({ reverse: true, timeout: 10_000 });
      expect(await rowNames()).toEqual(['Premiers pas', 'Gardée']);
      expect(await held()).toEqual(['Premiers pas[]', 'Gardée[]']);

      for (const node of (await bridge.httpTree()).collections) {
        await bridge.deleteHttpItem({ kind: 'collection', id: node.collection.id });
      }
    });

    /**
     * On a library with no collection, saving a first request answered « créez-en une dans le
     * rail » and could only be cancelled. The dialog creates what it lacks.
     */
    it('saves a first request by naming its collection, then others into a new folder and a new collection', async () => {
      for (const node of (await bridge.httpTree()).collections) {
        await bridge.deleteHttpItem({ kind: 'collection', id: node.collection.id });
      }
      await press('1', ['Control']);
      await $(testid('http-page')).waitForExist({ reverse: true, timeout: 10_000 });
      await press('3', ['Control']);
      await $(testid('http-page')).waitForDisplayed({ timeout: 10_000 });
      if (!(await $(testid('http-rail')).isExisting())) await press('b', ['Control']);
      await $(testid('http-rail-empty')).waitForDisplayed({ timeout: 10_000 });
      const held = async () => outline(await bridge.httpTree());
      const ends = (names: string[], expected: string[]) =>
        names.length === expected.length && names.every((name, at) => name.endsWith(expected[at]!));
      const begin = async () => {
        await $(testid('http-tab-new')).click();
        await press('s', ['Control']);
        await $(testid('http-save-dialog')).waitForDisplayed({ timeout: 10_000 });
      };
      const save = async () => {
        await $(testid('http-save-submit')).click();
        await $(testid('http-save-dialog')).waitForExist({ reverse: true, timeout: 10_000 });
      };

      // « Nouvelle requête », a URL, Ctrl+S: nothing to choose among, a collection to name.
      await $(testid('http-tab-new')).click();
      await setField(testid('http-url'), 'https://api.exemple.fr/sante');
      await press('s', ['Control']);
      await $(testid('http-save-dialog')).waitForDisplayed({ timeout: 10_000 });
      expect(await $(testid('http-save-place')).isExisting()).toBe(false);
      // Left without a name it is refused in place, and nothing is written.
      await $(testid('http-save-submit')).click();
      await $(testid('http-save-missing')).waitForDisplayed({ timeout: 10_000 });
      expect(await $(testid('http-save-collection')).getAttribute('aria-invalid')).toBe('true');
      expect(await held()).toEqual([]);
      await setField(testid('http-save-name'), 'Santé');
      await setField(testid('http-save-collection'), 'Boutique');
      await save();
      // One gesture: the collection unfolded in the rail, the request in it, lit, with its URL.
      await eventually(
        rowNames,
        (names) => ends(names, ['Boutique', 'Santé']),
        'the collection and its request',
      );
      expect(await held()).toEqual(['Boutique[Santé]']);
      expect(
        await readEach(`${testid('http-node-name')}[aria-current="true"]`, 'text').then((names) =>
          ends(names, ['Santé']),
        ),
      ).toBe(true);
      expect(await $(testid('http-url')).getValue()).toBe('https://api.exemple.fr/sante');
      expect(await $(testid('http-tab-dirty')).isExisting()).toBe(false);

      // With a collection there, a new folder under it — folded, and unfolded to show what it got.
      const twisty = `${testid('http-node')}[data-kind="collection"] ${testid('http-node-twisty')}`;
      await $(twisty).click();
      await eventually(rowNames, (names) => ends(names, ['Boutique']), 'the collection folded');
      await begin();
      await $(testid('http-save-new-folder')).click();
      await setField(testid('http-save-name'), 'Lister');
      await setField(testid('http-save-folder'), 'Commandes');
      await save();
      await eventually(
        rowNames,
        (names) => ends(names, ['Boutique', 'Santé', 'Commandes', 'Lister']),
        'the folder and its request, shown',
      );
      expect(await held()).toEqual(['Boutique[Santé Commandes[Lister]]']);

      // And a new collection from the end of the menu of places.
      await begin();
      await $(testid('http-save-place')).click();
      await $(
        `${testid('choice-panel-http-save-place')} ${testid('choice-option')}[data-option-id="new-collection"]`,
      ).click();
      await setField(testid('http-save-name'), 'Chercher');
      await setField(testid('http-save-collection'), 'Catalogue');
      await save();
      await eventually(held, (seen) => seen.length === 2, 'the second collection');
      expect(await held()).toEqual(['Boutique[Santé Commandes[Lister]]', 'Catalogue[Chercher]']);

      const tabCount = async () => (await $$(testid('http-tab')).getElements()).length;
      for (let left = await tabCount(); left > 0; left = await tabCount()) {
        await $(`${testid('http-tab')}:last-of-type ${testid('http-tab-close')}`).click();
        await eventually(tabCount, (count) => count === left - 1, 'a saved tab closed without a question');
      }
      for (const node of (await bridge.httpTree()).collections) {
        await bridge.deleteHttpItem({ kind: 'collection', id: node.collection.id });
      }
    });

    /**
     * A request typed in a tab and never saved, or a saved one with changes, was gone once
     * the page went away — and nothing said so. Whatever takes the page away asks first. Here
     * it is a library being created; the switch itself is in `25-libraries`, and a quit ends
     * the process every spec file shares: the Rust tests of `desktop` hold that path.
     */
    it('asks before another library takes the page: saves what has a place, and stays when told to', async () => {
      const api = await bridge.createHttpCollection('Avant de partir');
      const saved = await bridge.createHttpRequest(draft(api.id, null, 'Modifiée'));
      await press('1', ['Control']);
      await $(testid('http-page')).waitForExist({ reverse: true, timeout: 10_000 });
      await press('3', ['Control']);
      await $(testid('http-page')).waitForDisplayed({ timeout: 10_000 });
      if (!(await $(testid('http-rail')).isExisting())) await press('b', ['Control']);
      const tab = `${testid('http-tab')}[data-key="${saved.id}"]`;
      const libraries = async () => (await bridge.listLibraries()).libraries.length;
      const before = await libraries();
      const askToCreate = async () => {
        await $(testid('file-menu')).click();
        await $(testid('file-libraries')).click();
        await setField(testid('library-new-name'), 'Jamais créée');
        await $(testid('library-create')).click();
        await $(`${testid('http-leave-dialog')}[data-reason="switch"]`).waitForDisplayed({ timeout: 10_000 });
      };
      const stay = async () => {
        await $(testid('http-leave-cancel')).click();
        await $(testid('http-leave-dialog')).waitForExist({ reverse: true, timeout: 10_000 });
        await $(testid('libraries-close')).click();
        await $(testid('library-row')).waitForExist({ reverse: true, timeout: 10_000 });
      };

      // A saved request with a URL typed and not saved, and a request never saved at all.
      await $(`${testid('http-node')}[data-id="${saved.id}"] ${testid('http-node-name')}`).click();
      await $(tab).waitForDisplayed({ timeout: 10_000 });
      await setField(testid('http-url'), 'https://api.exemple.fr/pas-encore');
      await $(`${tab} ${testid('http-tab-dirty')}`).waitForExist({ timeout: 10_000 });
      await $(testid('http-tab-new')).click();
      const tabCount = async () => (await $$(testid('http-tab')).getElements()).length;
      await eventually(tabCount, (count) => count === 2, 'the request never saved');

      // Both are named before anything is created; « Annuler » leaves them, and the library, alone.
      await askToCreate();
      expect((await readEach(testid('http-leave-name'), 'text'))[0]).toBe('Modifiée');
      expect((await $$(testid('http-leave-name')).getElements()).length).toBe(2);
      expect(await libraries()).toBe(before);
      await stay();
      expect(await libraries()).toBe(before);
      expect(await $(`${tab} ${testid('http-tab-dirty')}`).isExisting()).toBe(true);
      expect((await bridge.httpRequest(saved.id)).document.url).toBe('https://api.exemple.fr/Modifiée');

      // « Tout enregistrer »: the one that has a place is written; the other is asked where
      // it goes, above the question — and not placing it comes back to the question.
      await askToCreate();
      await $(testid('http-leave-save')).click();
      await $(testid('http-save-dialog')).waitForDisplayed({ timeout: 10_000 });
      expect((await bridge.httpRequest(saved.id)).document.url).toBe('https://api.exemple.fr/pas-encore');
      await $(testid('http-save-cancel')).click();
      await $(testid('http-leave-dialog')).waitForDisplayed({ timeout: 10_000 });
      expect((await $$(testid('http-leave-name')).getElements()).length).toBe(1);
      await stay();
      expect(await libraries()).toBe(before);
      expect(await $(`${tab} ${testid('http-tab-dirty')}`).isExisting()).toBe(false);

      for (let left = await tabCount(); left > 0; left = await tabCount()) {
        await $(`${testid('http-tab')}:last-of-type ${testid('http-tab-close')}`).click();
        if (await $(testid('http-close-discard')).isExisting()) await $(testid('http-close-discard')).click();
        await eventually(tabCount, (count) => count === left - 1, 'a tab closed');
      }
      await bridge.deleteHttpItem({ kind: 'collection', id: api.id });
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
        await choose((await named('Commandes'))!, 'newHttp');
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
     * Deleting a folder closed the tabs of its requests with what was typed in them, without a
     * word: the confirmation counted what Rust deletes, and only the front knows what is unsaved.
     */
    it('warns that a deletion takes an open request with unsaved changes, and keeps it when cancelled', async () => {
      const api = await bridge.createHttpCollection('Pertes');
      const folder = await bridge.createHttpFolder(api.id, null, 'Brouillons');
      const edited = await bridge.createHttpRequest(draft(api.id, folder.id, 'Ébauche'));
      const calm = await bridge.createHttpRequest(draft(api.id, folder.id, 'Calme'));
      await press('1', ['Control']);
      await $(testid('http-page')).waitForExist({ reverse: true, timeout: 10_000 });
      await press('3', ['Control']);
      await $(testid('http-page')).waitForDisplayed({ timeout: 10_000 });
      if (!(await $(testid('http-rail')).isExisting())) await press('b', ['Control']);
      const node = (id: string) => `${testid('http-node')}[data-id="${id}"]`;
      const tab = (id: string) => `${testid('http-tab')}[data-key="${id}"]`;
      const ask = async (id: string) => {
        await choose(await $(node(id)).getElement(), 'delete');
        await $(testid('http-delete-confirm')).waitForDisplayed({ timeout: 10_000 });
      };
      const cancel = async () => {
        await $(testid('http-delete-cancel')).click();
        await $(testid('http-delete-confirm')).waitForExist({ reverse: true, timeout: 10_000 });
      };

      // One request open and left as it is, one open with a URL typed and not saved.
      await $(`${node(calm.id)} ${testid('http-node-name')}`).click();
      await $(tab(calm.id)).waitForDisplayed({ timeout: 10_000 });
      await $(`${node(edited.id)} ${testid('http-node-name')}`).click();
      await $(tab(edited.id)).waitForDisplayed({ timeout: 10_000 });
      await setField(testid('http-url'), 'https://api.exemple.fr/pas-enregistree');
      await $(`${tab(edited.id)} ${testid('http-tab-dirty')}`).waitForExist({ timeout: 10_000 });

      // The folder: what Rust counts, and beside it the one tab that would lose its changes.
      await ask(folder.id);
      expect(await $(testid('http-delete-count')).isExisting()).toBe(true);
      expect(await readEach(testid('http-delete-unsaved-name'), 'text')).toEqual(['Ébauche']);
      expect((await $$(testid('http-delete-confirm')).getElements()).length).toBe(1);
      await cancel();
      // Cancelled: the tab is there, modified, with what was typed.
      await $(tab(edited.id)).click();
      expect(await $(testid('http-url')).getValue()).toBe('https://api.exemple.fr/pas-enregistree');
      expect(await $(`${tab(edited.id)} ${testid('http-tab-dirty')}`).isExisting()).toBe(true);
      expect(outline(await bridge.httpTree())).toContain('Pertes[Brouillons[Ébauche Calme]]');

      // The whole collection says it too; the request's own row says it of itself, without a list.
      await ask(api.id);
      expect(await readEach(testid('http-delete-unsaved-name'), 'text')).toEqual(['Ébauche']);
      await cancel();
      await ask(edited.id);
      expect(await $(testid('http-delete-unsaved')).isExisting()).toBe(true);
      expect(await readEach(testid('http-delete-unsaved-name'), 'text')).toEqual([]);
      await cancel();

      // A tab that is open and not modified closes as before, without a word.
      await ask(calm.id);
      expect(await $(testid('http-delete-unsaved')).isExisting()).toBe(false);
      await $(testid('http-delete-submit')).click();
      await $(tab(calm.id)).waitForExist({ reverse: true, timeout: 10_000 });

      // Told, and confirmed: the folder goes, and its tab with it.
      await ask(folder.id);
      await $(testid('http-delete-submit')).click();
      await $(tab(edited.id)).waitForExist({ reverse: true, timeout: 10_000 });
      expect(outline(await bridge.httpTree())).toContain('Pertes[]');
      await bridge.deleteHttpItem({ kind: 'collection', id: api.id });
    });

    /**
     * Moving a row with `Alt` and the arrows worked, and was hard to follow: after a press,
     * nothing said which folder the row was now in. Each move is marked on the row and on what
     * holds it, and said; one that cannot be made is said too.
     */
    it('follows a request moved with the keys: into a folder, out of it, into a folded one', async () => {
      const api = await bridge.createHttpCollection('Déplacements');
      const folded = await bridge.createHttpFolder(api.id, null, 'Repliée');
      await bridge.createHttpRequest(draft(api.id, folded.id, 'Résidente'));
      const open = await bridge.createHttpFolder(api.id, null, 'Ouverte');
      const moved = await bridge.createHttpRequest(draft(api.id, null, 'Voyageuse'));
      await press('1', ['Control']);
      await $(testid('http-page')).waitForExist({ reverse: true, timeout: 10_000 });
      await press('3', ['Control']);
      await $(testid('http-page')).waitForDisplayed({ timeout: 10_000 });
      if (!(await $(testid('http-rail')).isExisting())) await press('b', ['Control']);
      const row = `${testid('http-node')}[data-id="${moved.id}"]`;
      await $(row).waitForDisplayed({ timeout: 10_000 });

      /** What the rail shows of the last move, read in one pass: the marks last a moment. */
      const shown = () =>
        browser.execute(
          (node: string, status: string, id: string) => {
            const said = document.querySelector(status)!;
            const box = document.querySelector(`${node}[data-id="${id}"]`)?.getBoundingClientRect();
            const rail = document.querySelector('[data-testid="http-rail"]')!.getBoundingClientRect();
            return {
              marks: Object.fromEntries(
                [...document.querySelectorAll(`${node}[data-moved]`)].map((element) => [
                  element.getAttribute('data-id'),
                  element.getAttribute('data-moved'),
                ]),
              ),
              said: said.getAttribute('data-said'),
              sentence: (said.textContent ?? '').trim(),
              inView: box !== undefined && box.top >= rail.top && box.bottom <= rail.bottom,
              focused: document.activeElement?.closest(node)?.getAttribute('data-id') ?? null,
            };
          },
          testid('http-node'),
          testid('http-rail-moved'),
          moved.id,
        );
      const parentOf = async () => (await bridge.httpRequest(moved.id)).folderId;
      /** One press, then the row marked, its new parent marked, the sentence, and where Rust holds it. */
      const move = async (
        key: string,
        said: string,
        parent: string,
        beside: string,
        folderId: string | null,
      ) => {
        await press(key, ['Alt']);
        const seen = await eventually(
          shown,
          (now) => now.said === said && now.marks[moved.id] === 'row' && now.focused === moved.id,
          `the move said as « ${said} »`,
        );
        expect(seen.marks).toEqual({ [moved.id]: 'row', [parent]: 'parent' });
        expect(seen.sentence).toContain('Voyageuse');
        expect(seen.sentence).toContain(beside);
        expect(seen.inView).toBe(true);
        expect(await parentOf()).toBe(folderId);
      };

      await browser.execute(
        (selector: string) => (document.querySelector(selector) as HTMLElement | null)?.focus(),
        `${row} ${testid('http-node-name')}`,
      );
      // Into the folder above it, out of it again, then up past it.
      await move('ArrowRight', 'into', open.id, 'Ouverte', open.id);
      await move('ArrowLeft', 'root', api.id, 'Déplacements', null);
      await move('ArrowUp', 'before', api.id, 'Ouverte', null);

      // The folder now above it is folded: it opens to show what it receives.
      const twisty = `${testid('http-node')}[data-id="${folded.id}"] ${testid('http-node-twisty')}`;
      await $(twisty).click();
      await eventually(
        () => $(twisty).getAttribute('aria-expanded'),
        (expanded) => expanded === 'false',
        'the folder folded',
      );
      await browser.execute(
        (selector: string) => (document.querySelector(selector) as HTMLElement | null)?.focus(),
        `${row} ${testid('http-node-name')}`,
      );
      await move('ArrowRight', 'into', folded.id, 'Repliée', folded.id);
      expect(await $(twisty).getAttribute('aria-expanded')).toBe('true');

      // Last of its folder: going down cannot be made, and that is said rather than nothing.
      const before = outline(await bridge.httpTree());
      await press('ArrowDown', ['Alt']);
      const refused = await eventually(shown, (now) => now.said === 'refused.last', 'the refusal said');
      expect(refused.sentence).toContain('Voyageuse');
      expect(outline(await bridge.httpTree())).toEqual(before);
      expect(before).toContain('Déplacements[Repliée[Résidente Voyageuse] Ouverte[]]');

      // The order is Rust's: still there once the front end is rebuilt.
      await reloadCanvas();
      await $(testid('http-page')).waitForDisplayed({ timeout: 10_000 });
      await $(row).waitForDisplayed({ timeout: 10_000 });
      const names = await rowNames();
      const at = (name: string) => names.findIndex((seen) => seen.endsWith(name));
      expect([at('Repliée'), at('Résidente'), at('Voyageuse'), at('Ouverte')]).toEqual([
        at('Repliée'),
        at('Repliée') + 1,
        at('Repliée') + 2,
        at('Repliée') + 3,
      ]);

      await bridge.deleteHttpItem({ kind: 'collection', id: api.id });
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

      await reloadCanvas();
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
        // Read as they are: a value trimmed here let « ␠1 » pass for « 1 » (HRQ-06).
        const cells = async () => [
          await $(name).getValue(),
          await $(`${row} ${testid('http-kv-value')}`).getValue(),
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
        // The sentence alone, without Rust's technical line under it (HSC-17).
        expect(await $(testid('error-detail')).isExisting()).toBe(false);
        expect(await $(testid('http-settings-dialog')).isDisplayed()).toBe(true);
        expect(await kept()).toEqual([]);
        await $(testid('error-dismiss')).click();

        await setField(name, 'X-Recette');
        await $(`${row}[data-fault]`).waitForExist({ reverse: true, timeout: 10_000 });
        await $(testid('http-settings-save')).click();
        await $(testid('http-settings-dialog')).waitForExist({ reverse: true, timeout: 10_000 });
        expect((await kept()).map((header) => [header.key, header.value])).toEqual([['X-Recette', '1']]);

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

    it('keeps the query and its table in step, takes a header typed or picked from its list, and saves on Ctrl+S', async () => {
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
      // Picked from the names' list as WebView2 does it: a keydown with no key, then the value.
      await browser.execute(
        (selector) => {
          const field = document.querySelector<HTMLInputElement>(selector);
          if (!field) return;
          field.dispatchEvent(new Event('keydown', { bubbles: true }));
          field.value = 'Content-Length';
          field.dispatchEvent(new Event('input', { bubbles: true }));
          field.dispatchEvent(new Event('keyup', { bubbles: true }));
        },
        `${testid('http-headers-table')} ${testid('http-kv-row')}:last-child ${testid('http-kv-key')}`,
      );
      await eventually(
        () => readEach(`${testid('http-headers-table')} ${testid('http-kv-key')}`, 'value'),
        (keys) => keys.join() === 'Accept,Content-Length,',
        'the name picked from the list in its row',
      );
      expect(await $(testid('error-banner')).isExisting()).toBe(false);
      await press('s', ['Control']);
      await $(testid('http-tab-dirty')).waitForExist({ reverse: true, timeout: 10_000 });

      const saved = await bridge.httpRequest(created.id);
      expect(saved.document.url).toBe('{{baseUrl}}/invoices?page=1&limit=50');
      expect(saved.document.params?.map((row) => row.key)).toEqual(['page', 'limit']);
      expect(saved.document.headers?.map((row) => row.key)).toEqual(['Accept', 'Content-Length']);

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
    /**
     * Three things about the pane a request answers in: the line between the two could not
     * be moved, an answer could not be put away, and « 200 OK » sat on a tint too light to
     * read as a signal.
     */
    it('moves the line between a request and its answer, clears an answer, and says the family of a status', async () => {
      await bridge.clearHttpHistory();
      const server = createServer((request, response) => {
        response.writeHead(Number((request.url ?? '/200').slice(1)), { 'Content-Type': 'application/json' });
        response.end('{"ok":true}');
      });
      await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve));
      const { port } = server.address() as AddressInfo;
      try {
        const api = await bridge.createHttpCollection('Volet');
        const created = await bridge.createHttpRequest({
          ...draft(api.id, null, 'Statuts'),
          document: { url: `http://127.0.0.1:${port}/200`, description: '' },
        });
        await press('1', ['Control']);
        await $(testid('http-page')).waitForExist({ reverse: true, timeout: 10_000 });
        await press('3', ['Control']);
        await $(testid('http-page')).waitForDisplayed({ timeout: 10_000 });
        if (!(await $(testid('http-rail')).isExisting())) await press('b', ['Control']);
        await $(`${testid('http-node')}[data-id="${created.id}"] ${testid('http-node-name')}`).click();
        await $(testid('http-response-idle')).waitForDisplayed({ timeout: 10_000 });

        /** The two heights, the box they share and the share the line says, read in one pass. */
        const split = () =>
          browser.execute(
            (editor: string, pane: string, line: string) => ({
              request: Math.round(
                document.querySelector(editor)!.parentElement!.getBoundingClientRect().height,
              ),
              response: Math.round(
                document.querySelector(pane)!.parentElement!.getBoundingClientRect().height,
              ),
              box: Math.round(document.querySelector(line)!.parentElement!.getBoundingClientRect().height),
              share: Number(document.querySelector(line)!.getAttribute('data-share')),
            }),
            testid('http-request-editor'),
            testid('http-response'),
            testid('http-split-handle'),
          );
        const onLine = async (key: string) => {
          await browser.execute(
            (line: string) => (document.querySelector(line) as HTMLElement | null)?.focus(),
            testid('http-split-handle'),
          );
          // Through `press`: the embedded driver has no `Home` nor `End` to send.
          await press(key);
        };
        /**
         * One key on the line, then where it leads. ⚠️ Waited for key by key, and for the panes
         * as well as the share: the page redraws after a key, not during it, and a share read
         * alone was answered before the two heights had followed it.
         */
        const move = async (key: string) => {
          const before = await split();
          await onLine(key);
          return eventually(
            split,
            (now) => now.share !== before.share && now.request !== before.request,
            `the line moved by ${key} from ${JSON.stringify(before)}`,
          );
        };
        // Whatever a run before left, it starts from the automatic share.
        await onLine('Enter');
        const auto = await eventually(split, (now) => now.share === 0, 'the automatic share');

        // The arrows move the line: the request taller, its answer shorter by as much.
        const once = await move('ArrowDown');
        const lower = await move('ArrowDown');
        expect(lower.share).toBeGreaterThan(once.share);
        expect(lower.request).toBeGreaterThan(auto.request);
        expect(lower.response).toBeLessThan(auto.response);
        expect(lower.request + lower.response).toBe(auto.request + auto.response);

        // Each end is a floor: the answer keeps its status line, the request its own line and tabs.
        const least = await move('End');
        expect(least.response).toBeGreaterThanOrEqual(158);
        expect(least.response).toBeLessThanOrEqual(164);
        await onLine('ArrowDown');
        expect(await split()).toEqual(least);
        const most = await move('Home');
        expect(most.request).toBeGreaterThanOrEqual(146);
        expect(most.request).toBeLessThanOrEqual(152);
        await onLine('ArrowUp');
        expect(await split()).toEqual(most);

        // The share is the area's, kept with the application: a trip to the notes, then a refresh.
        await move('ArrowDown');
        await move('ArrowDown');
        const kept = await move('ArrowDown');
        await press('1', ['Control']);
        await $(testid('http-page')).waitForExist({ reverse: true, timeout: 10_000 });
        await press('3', ['Control']);
        await $(testid('http-split-handle')).waitForExist({ timeout: 10_000 });
        expect(await eventually(split, (now) => now.share === kept.share, 'the share after a trip')).toEqual(
          kept,
        );
        await preferenceWritten('devnotes.httpRequestShare', String(kept.share));
        await reloadCanvas();
        await $(testid('http-page')).waitForDisplayed({ timeout: 10_000 });
        await $(testid('http-split-handle')).waitForExist({ timeout: 10_000 });
        expect(
          (await eventually(split, (now) => now.share === kept.share, 'the share after a refresh')).share,
        ).toBe(kept.share);

        // A status carries its family, for what cannot read a colour.
        const family = () => $(testid('http-response-status')).getAttribute('data-family');
        for (const [status, expected] of [
          [200, '2xx'],
          [404, '4xx'],
          [500, '5xx'],
        ] as const) {
          await setField(testid('http-url'), `http://127.0.0.1:${port}/${status}`);
          await $(testid('http-send')).click();
          await eventually(
            () => $(testid('http-response-status')).getText(),
            (text) => text.startsWith(String(status)),
            `the answer ${status}`,
          );
          expect(await family()).toBe(expected);
        }

        // « Effacer » puts the answer away: the pane waits again, the history kept the three sends.
        await $(testid('http-response-clear')).click();
        await $(testid('http-response-idle')).waitForDisplayed({ timeout: 10_000 });
        expect(await $(testid('http-response-status')).isExisting()).toBe(false);
        await $(testid('http-history-open')).click();
        await eventually(
          () => readEach(testid('http-history-status'), 'text'),
          (statuses) => statuses.length === 3,
          'the three sends still in the history',
        );
        await $(testid('http-history-open')).click();
        await $(testid('http-history')).waitForExist({ reverse: true, timeout: 10_000 });

        // Enter on the line is its double-click: back to the automatic share.
        await onLine('Enter');
        await eventually(
          split,
          (now) => now.share === 0 && now.request === auto.request,
          'the automatic share again',
        );

        const tab = `${testid('http-tab')}[data-key="${created.id}"]`;
        await $(`${tab} ${testid('http-tab-close')}`).click();
        await $(testid('http-close-discard')).click();
        await $(tab).waitForExist({ reverse: true, timeout: 10_000 });
        await bridge.deleteHttpItem({ kind: 'collection', id: api.id });
      } finally {
        server.close();
      }
    });

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
    /**
     * A query missing a brace was sent without a word and answered by a 400 whose errors waited
     * in a tab nothing pointed to; and the variables were typed after the query, in its box.
     */
    it('warns of a brace left open and lands on the errors, then names a variable without a value and adds it', async () => {
      const bodies: string[] = [];
      const server = createServer((request, response) => {
        let body = '';
        request.on('data', (chunk: Buffer) => (body += chunk.toString()));
        request.on('end', () => {
          bodies.push(body);
          const sent = JSON.parse(body) as { query: string; variables?: { code?: string | null } };
          if (!sent.query.trim().endsWith('} }')) {
            response.writeHead(400, { 'Content-Type': 'application/json' });
            response.end(
              '{"errors":[{"message":"Syntax Error: Expected Name, found <EOF>.","locations":[{"line":1,"column":58}]}]}',
            );
            return;
          }
          response.writeHead(200, { 'Content-Type': 'application/json' });
          response.end(JSON.stringify({ data: { country: { name: sent.variables?.code ?? null } } }));
        });
      });
      await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve));
      const { port } = server.address() as AddressInfo;
      try {
        const open = 'query Country($code: ID!) { country(code: $code) { name }';
        const api = await bridge.createHttpCollection('Pays');
        const created = await bridge.createHttpRequest({
          ...draft(api.id, null, 'Pays'),
          kind: 'graphql',
          document: { url: `http://127.0.0.1:${port}/graphql`, description: '', graphql: { query: open } },
        });
        await press('1', ['Control']);
        await $(testid('http-page')).waitForExist({ reverse: true, timeout: 10_000 });
        await press('3', ['Control']);
        await $(testid('http-page')).waitForDisplayed({ timeout: 10_000 });
        if (!(await $(testid('http-rail')).isExisting())) await press('b', ['Control']);
        await $(`${testid('http-node')}[data-id="${created.id}"] ${testid('http-node-name')}`).click();
        await $(testid('http-graphql-query')).waitForDisplayed({ timeout: 10_000 });

        // Said where it is typed — and sent all the same: the server stays the judge.
        await $(testid('http-graphql-query-problem')).waitForDisplayed({ timeout: 10_000 });
        expect(await $(testid('http-graphql-query-problem')).getAttribute('data-fault')).toBe(
          'unclosedBrace',
        );
        await $(testid('http-send')).click();
        await $(testid('http-response-errors')).waitForDisplayed({ timeout: 10_000 });
        expect(await $(testid('http-response-status')).getText()).toBe('400 Bad Request');
        expect(await $(testid('http-response-graphql-errors')).isDisplayed()).toBe(true);
        expect(await $(testid('http-response-error')).getText()).toContain('Syntax Error');

        // The brace closed, what is left to say is the variable the operation declares.
        await setField(testid('http-graphql-query'), `${open} }`);
        await $(testid('http-graphql-query-problem')).waitForExist({ reverse: true, timeout: 10_000 });
        await $(testid('http-graphql-missing')).waitForDisplayed({ timeout: 10_000 });
        expect(
          await readEach(`${testid('http-graphql-missing')} ${testid('http-graphql-variable')}`, 'text'),
        ).toEqual(['$code']);
        await $(testid('http-graphql-add')).click();
        await eventually(
          () => $(testid('http-graphql-variables')).getValue(),
          (variables) => variables.includes('"code": null'),
          'the variable added to the object',
        );
        await $(testid('http-graphql-missing')).waitForExist({ reverse: true, timeout: 10_000 });

        await setField(testid('http-graphql-variables'), '{ "code": "FR" }');
        await $(testid('http-send')).click();
        await eventually(
          () => $(testid('http-response-status')).getText(),
          (status) => status === '200 OK',
          'the answer to the query that reads',
        );
        // An answer with its data opens on it, no error beside its status.
        expect(await $(testid('http-response-pretty')).getText()).toContain('"name": "FR"');
        expect(await $(testid('http-response-graphql-errors')).isExisting()).toBe(false);
        expect((JSON.parse(bodies.at(-1)!) as { variables: unknown }).variables).toEqual({ code: 'FR' });

        const tab = `${testid('http-tab')}[data-key="${created.id}"]`;
        await $(`${tab} ${testid('http-tab-close')}`).click();
        await $(testid('http-close-discard')).click();
        await $(tab).waitForExist({ reverse: true, timeout: 10_000 });
        await bridge.deleteHttpItem({ kind: 'collection', id: api.id });
      } finally {
        server.close();
      }
    });

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

    /**
     * Fourteen sends to one host looked alike in a list that could not be widened, one send
     * could not be removed alone, and « Rouvrir » opened a stranger instead of its request.
     */
    it('widens the history, deletes one send and puts it back, and reopens a send in its own request', async () => {
      const server = createServer((_, response) => {
        response.writeHead(200, { 'Content-Type': 'text/plain' });
        response.end('ok');
      });
      await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve));
      const { port } = server.address() as AddressInfo;
      try {
        await bridge.clearHttpHistory();
        const api = await bridge.createHttpCollection('Journal');
        const created = await bridge.createHttpRequest({
          ...draft(api.id, null, 'Lire'),
          document: { url: `http://127.0.0.1:${port}/first`, description: '' },
        });
        await press('1', ['Control']);
        await $(testid('http-page')).waitForExist({ reverse: true, timeout: 10_000 });
        await press('3', ['Control']);
        await $(testid('http-page')).waitForDisplayed({ timeout: 10_000 });
        if (!(await $(testid('http-rail')).isExisting())) await press('b', ['Control']);
        await $(`${testid('http-node')}[data-id="${created.id}"] ${testid('http-node-name')}`).click();
        await $(testid('http-response-idle')).waitForDisplayed({ timeout: 10_000 });
        const tab = `${testid('http-tab')}[data-key="${created.id}"]`;
        const sends = async () => {
          await $(testid('http-send')).click();
          await $(testid('http-response-status')).waitForDisplayed({ timeout: 10_000 });
          await $(testid('http-response-sending')).waitForExist({ reverse: true, timeout: 10_000 });
        };
        const paths = () => readEach(testid('http-history-item-path'), 'text');
        const width = () =>
          browser.execute(
            (list: string) => Math.round(document.querySelector(list)!.getBoundingClientRect().width),
            testid('http-history-list'),
          );
        const nudge = async (key: 'ArrowLeft' | 'ArrowRight') => {
          await browser.execute(
            (edge: string) => (document.querySelector(edge) as HTMLElement | null)?.focus(),
            testid('http-history-edge'),
          );
          await browser.keys(Array.from({ length: 5 }, () => key));
        };

        // Two sends, the second saved as what the request now is.
        await sends();
        await setField(testid('http-url'), `http://127.0.0.1:${port}/second?page=2`);
        await sends();
        await press('s', ['Control']);
        await $(`${tab} ${testid('http-tab-dirty')}`).waitForExist({ reverse: true, timeout: 10_000 });

        // Each row ends on what tells it from the next; the list is widened, and stays so.
        await $(testid('http-history-open')).click();
        await eventually(
          paths,
          (seen) => seen.join() === '/second?page=2,/first',
          'the two sends, the newest first',
        );
        const narrow = await width();
        await nudge('ArrowRight');
        await eventually(width, (wide) => wide === narrow + 80, 'the list widened from its edge');
        // As the setting holds it: the edge says the width it stores.
        await preferenceWritten(
          'devnotes.historyListWidth',
          (await $(testid('http-history-edge')).getAttribute('aria-valuenow')) ?? '',
        );
        await reloadCanvas();
        await $(testid('http-page')).waitForDisplayed({ timeout: 10_000 });
        await $(testid('http-history')).waitForExist({ reverse: true, timeout: 10_000 });
        await $(testid('http-history-open')).click();
        await eventually(paths, (seen) => seen.length === 2, 'the history read again');
        expect(await width()).toBe(narrow + 80);

        // One send goes, and comes back to its place from the bar that follows.
        await $(`${testid('http-history-row')} ${testid('http-history-delete')}`).click();
        await eventually(paths, (seen) => seen.join() === '/first', 'the newest send gone');
        await $(testid('http-history-undo-restore')).click();
        await eventually(
          paths,
          (seen) => seen.join() === '/second?page=2,/first',
          'the send back in its place',
        );

        // The older send goes back into the request it came from: its tab, marked modified.
        await $(`${testid('http-history-row')}:nth-child(2) ${testid('http-history-item')}`).click();
        await $(`${testid('http-history-reopen')}[data-into="request"]`).waitForDisplayed({
          timeout: 10_000,
        });
        await $(testid('http-history-reopen')).click();
        await $(testid('http-history')).waitForExist({ reverse: true, timeout: 10_000 });
        await $(`${tab} ${testid('http-tab-dirty')}`).waitForExist({ timeout: 10_000 });
        expect(await $(testid('http-url')).getValue()).toBe(`http://127.0.0.1:${port}/first`);
        expect(await readEach(testid('http-tab'), '@data-key')).toEqual([created.id]);
        // Nothing is written until it is saved.
        expect((await bridge.httpRequest(created.id)).document.url).toBe(
          `http://127.0.0.1:${port}/second?page=2`,
        );

        await $(testid('http-history-open')).click();
        await $(testid('http-history-edge')).waitForExist({ timeout: 10_000 });
        await nudge('ArrowLeft');
        await eventually(width, (back) => back === narrow, 'the list back to its width');
        await $(testid('http-history-open')).click();
        await $(testid('http-history')).waitForExist({ reverse: true, timeout: 10_000 });

        await $(`${tab} ${testid('http-tab-close')}`).click();
        await $(testid('http-close-discard')).click();
        await $(tab).waitForExist({ reverse: true, timeout: 10_000 });
        await bridge.deleteHttpItem({ kind: 'collection', id: api.id });
        await bridge.clearHttpHistory();
      } finally {
        server.close();
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

    /**
     * `Ctrl+Entrée` in the message did nothing of what was meant, closing the tab of an open
     * socket cut it without a word, and `https://` in a socket failed « Erreur réseau ».
     */
    it('sends a message on Ctrl+Entrée, asks before closing a connected tab, and names the scheme a socket opens on', async () => {
      const echo = await echoSocket();
      try {
        const api = await bridge.createHttpCollection('Messages');
        const live = await bridge.createHttpRequest({
          ...draft(api.id, null, 'En direct'),
          kind: 'websocket',
          document: { url: `ws://127.0.0.1:${echo.port}/`, description: '' },
        });
        const secure = await bridge.createHttpRequest({
          ...draft(api.id, null, 'Mauvais schéma'),
          kind: 'websocket',
          document: { url: `https://127.0.0.1:${echo.port}/live`, description: '' },
        });
        await press('1', ['Control']);
        await $(testid('http-page')).waitForExist({ reverse: true, timeout: 10_000 });
        await press('3', ['Control']);
        await $(testid('http-page')).waitForDisplayed({ timeout: 10_000 });
        if (!(await $(testid('http-rail')).isExisting())) await press('b', ['Control']);
        await $(`${testid('http-node')}[data-id="${live.id}"] ${testid('http-node-name')}`).click();
        await $(testid('http-socket-connect')).waitForDisplayed({ timeout: 10_000 });
        const tab = `${testid('http-tab')}[data-key="${live.id}"]`;
        const phase = () => $(testid('http-socket-phase')).getAttribute('data-phase');
        const echoed = (text: string, what: string) =>
          eventually(
            () => readEach(testid('http-socket-entry'), 'text'),
            (rows) => rows.some((row) => row.includes(`echo: ${text}`)),
            what,
          );

        // Not connected: the key says so, and connects nothing behind one's back.
        await setField(testid('http-socket-message'), 'hello');
        await press('Enter', ['Control']);
        await $(testid('http-socket-not-open')).waitForDisplayed({ timeout: 10_000 });
        expect(await phase()).toBe('closed');

        await $(testid('http-socket-connect')).click();
        await $(`${testid('http-socket-phase')}[data-phase="open"]`).waitForDisplayed({ timeout: 10_000 });
        await $(testid('http-socket-message')).click();
        await press('Enter', ['Control']);
        await echoed('hello', 'the echo of the message the key sent');
        expect(echo.handshakes).toHaveLength(1);

        // Closing asks; « Annuler » keeps the connection: a further message is echoed.
        await $(`${tab} ${testid('http-tab-close')}`).click();
        await $(testid('http-close-connected')).waitForDisplayed({ timeout: 10_000 });
        expect(await $(testid('http-close-save')).isExisting()).toBe(false);
        await $(testid('http-close-cancel')).click();
        await $(testid('http-close-dialog')).waitForExist({ reverse: true, timeout: 10_000 });
        expect(await phase()).toBe('open');
        await setField(testid('http-socket-message'), 'again');
        await press('Enter', ['Control']);
        await echoed('again', 'a further echo on the connection that was kept');

        await $(`${tab} ${testid('http-tab-close')}`).click();
        await $(testid('http-close-discard')).click();
        await $(tab).waitForExist({ reverse: true, timeout: 10_000 });

        // `https://` is not dialled: said for what it is, its twin one click away.
        await $(`${testid('http-node')}[data-id="${secure.id}"] ${testid('http-node-name')}`).click();
        await $(testid('http-socket-connect')).waitForDisplayed({ timeout: 10_000 });
        await $(testid('http-socket-connect')).click();
        await $(`${testid('http-socket-failure')}[data-key="errors.httpWebsocketScheme"]`).waitForDisplayed({
          timeout: 10_000,
        });
        expect(echo.handshakes).toHaveLength(1);
        await $(testid('http-socket-twin')).click();
        await eventually(
          () => $(testid('http-url')).getValue(),
          (url) => url === `wss://127.0.0.1:${echo.port}/live`,
          'the twin written in the URL',
        );

        const other = `${testid('http-tab')}[data-key="${secure.id}"]`;
        await $(`${other} ${testid('http-tab-close')}`).click();
        await $(testid('http-close-discard')).click();
        await $(other).waitForExist({ reverse: true, timeout: 10_000 });
        await bridge.deleteHttpItem({ kind: 'collection', id: api.id });
      } finally {
        echo.close();
      }
    });

    it('keeps the cookies an answer sets, sends them back, masks them in the history, and empties the jar after saying how many', async () => {
      const heard: (string | undefined)[] = [];
      const server = createServer((request, response) => {
        heard.push(request.headers.cookie);
        if (request.url === '/login') response.setHeader('Set-Cookie', ['session=abc; Path=/; HttpOnly']);
        response.end('ok');
      });
      await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve));
      const { port } = server.address() as AddressInfo;
      try {
        await bridge.clearHttpHistory();
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

        // The history keeps neither side of it: what an answer sets is what the next request
        // sends. The cookie's name and what it applies to stay, its value reads as dots.
        const kept = (part: string) => `${testid('http-history')} ${part}`;
        await $(testid('http-history-open')).click();
        await $(`${testid('http-history-row')}:nth-child(2) ${testid('http-history-item')}`).click();
        await $(kept(`${testid('http-response-section')}[data-section="headers"]`)).click();
        const headers = await eventually(
          () => readEach(kept(`${testid('http-response-headers')} tr`), 'text'),
          (rows) => rows.some((row) => row.toLowerCase().includes('set-cookie')),
          'the headers of the answer that was kept',
        );
        expect(headers.find((row) => row.toLowerCase().includes('set-cookie'))).toContain(
          'session=••••••••; Path=/; HttpOnly',
        );
        expect(headers.join()).not.toContain('abc');
        await $(kept(`${testid('http-response-section')}[data-section="cookies"]`)).click();
        const cookies = await eventually(
          () => readEach(kept(testid('http-response-cookie')), 'text'),
          (rows) => rows.length === 1,
          'the cookie the answer set, in the history',
        );
        expect(cookies[0]).toContain('session');
        expect(cookies[0]).not.toContain('abc');
        await $(testid('http-history-open')).click();
        await $(testid('http-history')).waitForExist({ reverse: true, timeout: 10_000 });

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
        await bridge.clearHttpHistory();
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
