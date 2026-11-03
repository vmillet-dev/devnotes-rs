import { createServer } from 'node:http';
import type { AddressInfo } from 'node:net';

import { $, expect } from '@wdio/globals';

import { canvas } from '../pageobjects/canvas.page.js';
import { eventually, press, readEach, setField, testid } from '../support/app.js';
import { bridge } from '../support/bridge.js';

/** The HTTP area opened afresh, its rail shown: what a scenario wrote through Rust is read. */
async function openHttp(): Promise<void> {
  if (await $(testid('http-page')).isExisting()) {
    await press('1', ['Control']);
    await $(testid('http-page')).waitForExist({ reverse: true, timeout: 10_000 });
  }
  await press('3', ['Control']);
  await $(testid('http-page')).waitForDisplayed({ timeout: 10_000 });
  if (!(await $(testid('http-rail')).isExisting())) await press('b', ['Control']);
}

async function pick(id: string): Promise<void> {
  await $(testid('http-environment-picker')).click();
  await $(`${testid('http-environment-option')}[data-id="${id}"]`).click();
  await eventually(
    () => $(testid('http-environment-picker')).getAttribute('data-active'),
    (active) => active === id,
    `the environment ${id} active`,
  );
}

describe('HTTP environments', () => {
  before(async () => {
    await canvas.open();
  });

  after(async () => {
    await press('1', ['Control']);
    await $(testid('http-page')).waitForExist({ reverse: true, timeout: 10_000 });
  });

  /**
   * #485: Local, Staging, Production, each with its variables and a secret masked; the one a
   * request is sent with is picked in the tab bar, and a protected one asks before a POST.
   */
  it('keeps an environment and its secret, then asks before a POST leaves under it once protected', async () => {
    const heard: string[] = [];
    const server = createServer((request, response) => {
      heard.push(`${request.method} ${request.url}`);
      response.writeHead(200, { 'Content-Type': 'application/json' });
      response.end('{}');
    });
    await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve));
    const { port } = server.address() as AddressInfo;
    const api = await bridge.createHttpCollection('Environnements');
    const created = await bridge.createHttpRequest({
      collectionId: api.id,
      folderId: null,
      name: 'Créer un client',
      kind: 'http',
      method: 'POST',
      document: { url: `http://127.0.0.1:${port}/customers`, description: '' },
    });
    try {
      await openHttp();
      await $(testid('http-environments-open')).click();
      await $(testid('http-environments')).waitForDisplayed({ timeout: 10_000 });

      await $(testid('http-environment-create-open')).click();
      await setField(testid('http-environment-create-input'), 'Recette e2e');
      await $(testid('http-environment-create-submit')).click();
      await eventually(
        () => $(testid('http-environment-name')).getValue(),
        (name) => name === 'Recette e2e',
        'the new environment shown',
      );

      const rows = `${testid('http-environment-variables')} ${testid('http-variable-row')}`;
      await setField(`${rows}:last-child ${testid('http-variable-name')}`, 'apiToken');
      await setField(`${rows}:first-child ${testid('http-variable-value')}`, 'zq-e2e-token');
      await $(`${rows}:first-child ${testid('http-variable-kind')}`).click();
      await $(`${testid('choice-option')}[data-option-id="secret"]`).click();
      await eventually(
        () => $(`${rows}:first-child ${testid('http-variable-value')}`).getAttribute('type'),
        (type) => type === 'password',
        'the secret masked',
      );
      await $(`${rows}:first-child ${testid('http-variable-reveal')}`).click();
      expect(await $(`${rows}:first-child ${testid('http-variable-value')}`).getAttribute('type')).toBe(
        'text',
      );

      await $(testid('http-environment-protected')).click();
      const stored = await eventually(
        async () =>
          (await bridge.httpEnvironments()).environments.find((held) => held.name === 'Recette e2e'),
        (held) =>
          held?.protected === true &&
          held.variables.some(
            (row) => row.name === 'apiToken' && row.kind === 'secret' && row.value === 'zq-e2e-token',
          ),
        'the environment written, protected, with its secret',
      );
      expect(await $(testid('http-environment-protected-note')).isDisplayed()).toBe(true);

      // Back to the request, sent with it.
      await $(testid('http-environments-open')).click();
      await $(testid('http-environments')).waitForExist({ reverse: true, timeout: 10_000 });
      await $(`${testid('http-node')}[data-id="${created.id}"] ${testid('http-node-name')}`).click();
      await $(testid('http-request-editor')).waitForDisplayed({ timeout: 10_000 });
      await pick(stored!.id);

      await press('Enter', ['Control']);
      await $(testid('http-protected-confirm')).waitForDisplayed({ timeout: 10_000 });
      expect(await $(testid('http-protected-confirm')).getText()).toContain('Recette e2e');
      expect(heard).toEqual([]);
      await $(testid('http-protected-cancel')).click();
      await $(testid('http-protected-confirm')).waitForExist({ reverse: true, timeout: 10_000 });

      await $(testid('http-send')).click();
      await $(testid('http-protected-send')).click();
      await eventually(
        () => Promise.resolve(heard.join()),
        (said) => said === 'POST /customers',
        'the POST sent once the yes was given',
      );
      await $(testid('http-protected-confirm')).waitForExist({ reverse: true, timeout: 10_000 });

      // Deleted from its page: the tab bar sends with none.
      await $(testid('http-environment-picker')).click();
      await $(testid('http-environment-manage')).click();
      await $(testid('http-environments')).waitForDisplayed({ timeout: 10_000 });
      await $(`${testid('http-environment-entry')}[data-id="${stored!.id}"]`).click();
      await $(testid('http-environment-delete')).click();
      await $(testid('http-environment-delete-confirm')).click();
      await eventually(
        () => readEach(testid('http-environment-entry'), '@data-id'),
        (ids) => !ids.includes(stored!.id),
        'the environment gone from the list',
      );
      await $(testid('http-environments-open')).click();
      await eventually(
        () => $(testid('http-environment-picker')).getAttribute('data-active'),
        (active) => active === 'none',
        'no environment active',
      );

      const tab = `${testid('http-tab')}[data-key="${created.id}"]`;
      await $(`${tab} ${testid('http-tab-close')}`).click();
      await $(tab).waitForExist({ reverse: true, timeout: 10_000 });
      expect(await $(testid('error-banner')).isExisting()).toBe(false);
    } finally {
      server.close();
      await bridge.deleteHttpItem({ kind: 'collection', id: api.id });
    }
  });

  /**
   * #486: the nearest level wins, resolved by Rust when the request leaves — the environment's
   * host, the collection's tenant, the request's own page — and a name nothing sets stops it.
   */
  it('sends each variable as the nearest level sets it, and stops on one nothing sets', async () => {
    const heard: string[] = [];
    const server = createServer((request, response) => {
      heard.push(`${request.method} ${request.url}`);
      response.writeHead(200, { 'Content-Type': 'application/json' });
      response.end('{}');
    });
    await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve));
    const { port } = server.address() as AddressInfo;
    const api = await bridge.createHttpCollection('Variables');
    await bridge.saveHttpSettings(
      { kind: 'collection', id: api.id },
      { auth: { kind: 'none' }, headers: [], transport: {}, variables: [{ name: 'tenant', value: 'acme' }] },
    );
    const local = await bridge.createHttpEnvironment('Local e2e');
    await bridge.saveHttpEnvironment(local.id, {
      variables: [
        { name: 'baseUrl', value: `http://127.0.0.1:${port}` },
        { name: 'tenant', value: 'not-this-one' },
      ],
    });
    const created = await bridge.createHttpRequest({
      collectionId: api.id,
      folderId: null,
      name: 'Lister',
      kind: 'http',
      method: 'GET',
      document: {
        url: '{{baseUrl}}/t/{{tenant}}?page={{page}}',
        description: '',
        variables: [{ name: 'page', value: '2' }],
      },
    });
    try {
      await openHttp();
      await $(`${testid('http-node')}[data-id="${created.id}"] ${testid('http-node-name')}`).click();
      await $(testid('http-request-editor')).waitForDisplayed({ timeout: 10_000 });
      await pick(local.id);

      const chips = `${testid('http-used-variables')} ${testid('http-used-variable')}`;
      await eventually(
        () => readEach(chips, '@data-scope'),
        (scopes) => scopes.join() === 'environment,collection,request',
        'each variable in the colour of where it resolves from',
      );
      expect(await readEach(testid('http-url-variable'), '@data-scope')).toEqual([
        'environment',
        'collection',
        'request',
      ]);

      await $(testid('http-send')).click();
      await eventually(
        () => Promise.resolve(heard.join()),
        (said) => said === 'GET /t/acme?page=2',
        'the request sent with every variable filled',
      );

      await setField(testid('http-url'), '{{baseUrl}}/t/{{tenantId}}');
      await eventually(
        () => readEach(chips, '@data-scope'),
        (scopes) => scopes.join() === 'environment,none',
        'the name nothing sets, in red',
      );
      await $(testid('http-send')).click();
      await $(testid('http-response-failure')).waitForDisplayed({ timeout: 10_000 });
      expect(await $(testid('http-response-failure')).getText()).toContain('tenantId');
      expect(heard).toHaveLength(1);

      const tab = `${testid('http-tab')}[data-key="${created.id}"]`;
      await $(`${tab} ${testid('http-tab-close')}`).click();
      await $(testid('http-close-discard')).click();
      await $(tab).waitForExist({ reverse: true, timeout: 10_000 });
      await pick('none');
    } finally {
      server.close();
      await bridge.deleteHttpEnvironment(local.id);
      await bridge.deleteHttpItem({ kind: 'collection', id: api.id });
    }
  });

  /** #487: once the request left, a secret shows by its name — the timeline, the history. */
  it('shows a secret by its name in the timeline and the history, and the environment it left with', async () => {
    const heard: string[] = [];
    const server = createServer((request, response) => {
      heard.push(String(request.headers['x-token']));
      response.writeHead(200, { 'Content-Type': 'application/json' });
      response.end('{}');
    });
    await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve));
    const { port } = server.address() as AddressInfo;
    await bridge.clearHttpHistory();
    const api = await bridge.createHttpCollection('Secrets');
    const staging = await bridge.createHttpEnvironment('Staging e2e');
    await bridge.saveHttpEnvironment(staging.id, {
      variables: [{ name: 'token', value: 'zq-e2e-secret', kind: 'secret' }],
    });
    const created = await bridge.createHttpRequest({
      collectionId: api.id,
      folderId: null,
      name: 'Profil',
      kind: 'http',
      method: 'GET',
      document: {
        url: `http://127.0.0.1:${port}/me`,
        description: '',
        headers: [{ key: 'X-Token', value: '{{token}}' }],
      },
    });
    try {
      await openHttp();
      await $(`${testid('http-node')}[data-id="${created.id}"] ${testid('http-node-name')}`).click();
      await $(testid('http-request-editor')).waitForDisplayed({ timeout: 10_000 });
      await pick(staging.id);

      await $(testid('http-send')).click();
      await eventually(
        () => Promise.resolve(heard.join()),
        (said) => said === 'zq-e2e-secret',
        'the secret sent to the server',
      );
      await $(`${testid('http-response-section')}[data-section="timeline"]`).click();
      const sent = await $(testid('http-response-sent')).getText();
      expect(sent).toContain('X-Token: {{token}}');
      expect(sent).not.toContain('zq-e2e-secret');

      await $(testid('http-history-open')).click();
      await eventually(
        () => readEach(testid('http-history-environment'), 'text'),
        (names) => names.join() === 'Staging e2e',
        'the environment the send left with',
      );
      const entry = (await bridge.httpHistory(0))[0]?.items[0];
      const kept = JSON.stringify(await bridge.httpHistoryEntry(entry!.id));
      expect(kept).toContain('{{token}}');
      expect(kept).not.toContain('zq-e2e-secret');
      await $(testid('http-history-open')).click();
      await $(testid('http-history')).waitForExist({ reverse: true, timeout: 10_000 });

      const tab = `${testid('http-tab')}[data-key="${created.id}"]`;
      await $(`${tab} ${testid('http-tab-close')}`).click();
      await $(tab).waitForExist({ reverse: true, timeout: 10_000 });
      await pick('none');
    } finally {
      server.close();
      await bridge.deleteHttpEnvironment(staging.id);
      await bridge.deleteHttpItem({ kind: 'collection', id: api.id });
    }
  });
});
