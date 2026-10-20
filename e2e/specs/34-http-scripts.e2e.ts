import { createServer } from 'node:http';
import type { AddressInfo } from 'node:net';

import { $, browser, expect } from '@wdio/globals';

import { canvas } from '../pageobjects/canvas.page.js';
import { blur, checkedSegment, eventually, pickSegment, press, testid } from '../support/app.js';
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

const CODE = `${testid('http-script-code')} .cm-content`;

/** What the script's editor holds, as it draws its lines. */
function scriptText(): Promise<string> {
  return browser.execute(
    (selector: string) =>
      [...document.querySelectorAll(`${selector} .cm-line`)]
        .map((line) => (line.querySelector('.cm-placeholder') ? '' : (line.textContent ?? '')))
        .join('\n'),
    testid('http-script-code'),
  );
}

/** Through the page's own editing, which CodeMirror reads from the DOM as it reads a keyboard. */
async function typeScript(text: string): Promise<void> {
  await $(CODE).waitForExist({ timeout: 10_000 });
  await browser.execute(
    (selector: string, value: string) => {
      (document.querySelector(selector) as HTMLElement).focus();
      document.execCommand('selectAll');
      document.execCommand('insertText', false, value);
    },
    CODE,
    text,
  );
  await eventually(scriptText, (shown) => shown === text, 'the script editor to hold what was typed');
  await blur();
}

describe('HTTP scripts', () => {
  before(async () => {
    await canvas.open();
  });

  after(async () => {
    await press('1', ['Control']);
    await $(testid('http-page')).waitForExist({ reverse: true, timeout: 10_000 });
  });

  /**
   * #493: a script before the send signs the request, one after the answer keeps a value of it
   * in the active environment; a script that throws keeps the request in and says where.
   */
  it('changes the request before it leaves, keeps a value of the answer, and stops on a throw', async () => {
    const heard: string[] = [];
    const server = createServer((request, response) => {
      heard.push(`${request.method} ${request.url} ${String(request.headers['x-signed'] ?? '-')}`);
      response.writeHead(200, { 'Content-Type': 'application/json' });
      response.end('{"id":7}');
    });
    await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve));
    const { port } = server.address() as AddressInfo;
    const api = await bridge.createHttpCollection('Scripts');
    const staging = await bridge.createHttpEnvironment('Scripts e2e');
    const created = await bridge.createHttpRequest({
      collectionId: api.id,
      folderId: null,
      name: 'Commander',
      kind: 'http',
      method: 'GET',
      document: { url: `http://127.0.0.1:${port}/orders`, description: '' },
    });
    try {
      await openHttp();
      await $(`${testid('http-node')}[data-id="${created.id}"] ${testid('http-node-name')}`).click();
      await $(testid('http-request-editor')).waitForDisplayed({ timeout: 10_000 });
      await pick(staging.id);

      await $(`${testid('http-section')}[data-section="scripts"]`).click();
      await $(testid('http-scripts')).waitForDisplayed({ timeout: 10_000 });
      expect(await $(testid('http-script-order')).getText()).toContain('Commander');
      await typeScript("req.headers.set('X-Signed', 'yes');");
      await pickSegment('http-script-phase', 'after');
      await typeScript("env.set('orderId', String(res.json().id));");
      expect(
        await $(
          `${testid('http-section')}[data-section="scripts"] ${testid('http-section-count')}`,
        ).getText(),
      ).toBe('2');

      await press('Enter', ['Control']);
      await eventually(
        () => Promise.resolve(heard.join()),
        (said) => said === 'GET /orders yes',
        'the request sent as the script left it',
      );
      await eventually(
        () => $(testid('http-response-status')).getText(),
        (status) => status.startsWith('200'),
        'the answer shown',
      );
      await eventually(
        async () =>
          (await bridge.httpEnvironments()).environments
            .find((held) => held.id === staging.id)
            ?.variables.find((row) => row.name === 'orderId')?.value,
        (value) => value === '7',
        'the value the script kept, in the active environment',
      );

      // Saved with the request, both of them.
      await press('s', ['Control']);
      await eventually(
        async () => (await bridge.httpRequest(created.id)).document.scripts,
        (kept) =>
          kept?.before === "req.headers.set('X-Signed', 'yes');" &&
          kept.after === "env.set('orderId', String(res.json().id));",
        'the scripts saved with the request',
      );

      // A throw keeps the request in, and leads back to the line.
      await pickSegment('http-script-phase', 'before');
      await typeScript("throw new Error('zq-no-token');");
      await $(`${testid('http-section')}[data-section="headers"]`).click();
      await press('Enter', ['Control']);
      await $(testid('http-response-unsent')).waitForDisplayed({ timeout: 10_000 });
      expect(await $(testid('http-script-stop-message')).getText()).toContain('zq-no-token');
      expect(await $(testid('http-script-stop-where')).getText()).toContain('1');
      expect(heard).toHaveLength(1);
      await $(testid('http-script-open')).click();
      await $(testid('http-scripts')).waitForDisplayed({ timeout: 10_000 });
      expect(await checkedSegment('http-script-phase')).toBe('before');
      expect(await $(testid('error-banner')).isExisting()).toBe(false);
    } finally {
      server.close();
      await bridge.deleteHttpItem({ kind: 'collection', id: api.id });
      await bridge.deleteHttpEnvironment(staging.id);
    }
  });

  /**
   * #494: named tests run after the answer, the collection's for each of its requests; the
   * response's Tests tab counts them and says, for a failure, what was expected and what came.
   */
  it('runs the named tests after the answer and shows which passed', async () => {
    const server = createServer((_request, response) => {
      response.writeHead(402, { 'Content-Type': 'application/json' });
      response.end('{"currency":"usd"}');
    });
    await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve));
    const { port } = server.address() as AddressInfo;
    const api = await bridge.createHttpCollection('Tests');
    await bridge.saveHttpSettings(
      { kind: 'collection', id: api.id },
      {
        auth: { kind: 'none' },
        headers: [],
        transport: {},
        variables: [],
        scripts: {
          tests: "test('zq-trace', () => {\n  expect(res.headers).toHaveProperty('x-request-id');\n});",
        },
      },
    );
    const created = await bridge.createHttpRequest({
      collectionId: api.id,
      folderId: null,
      name: 'Payer',
      kind: 'http',
      method: 'GET',
      document: {
        url: `http://127.0.0.1:${port}/pay`,
        description: '',
        scripts: {
          tests:
            "test('zq-status', () => {\n  expect([200, 402]).toContain(res.status);\n});\n" +
            "pm.test('zq-currency', () => {\n  pm.expect(res.json().currency).to.equal('eur');\n});",
        },
      },
    });
    try {
      await openHttp();
      await $(`${testid('http-node')}[data-id="${created.id}"] ${testid('http-node-name')}`).click();
      await $(testid('http-request-editor')).waitForDisplayed({ timeout: 10_000 });
      expect(
        await $(`${testid('http-section')}[data-section="tests"] ${testid('http-section-count')}`).getText(),
      ).toBe('2');

      await press('Enter', ['Control']);
      await eventually(
        () => $(testid('http-response-tests-count')).getText(),
        (count) => count === '1/3',
        'the tests counted beside the body',
      );
      await $(`${testid('http-response-section')}[data-section="tests"]`).click();
      await $(testid('http-response-tests')).waitForDisplayed({ timeout: 10_000 });
      const results = await browser.execute(
        (selector: string) =>
          [...document.querySelectorAll(selector)].map(
            (test) =>
              `${test.querySelector('.name')?.textContent ?? ''} ${test.getAttribute('data-result') ?? ''}`,
          ),
        testid('http-test'),
      );
      expect(results).toEqual(['zq-status passed', 'zq-currency failed', 'zq-trace failed']);
      expect(await $(`${testid('http-test')}[data-result="failed"] .expected`).getText()).toBe('"eur"');
      expect(await $(`${testid('http-test')}[data-result="failed"] .actual`).getText()).toBe('"usd"');

      await $(testid('http-test-line')).click();
      await eventually(
        () => $(testid('http-scripts')).getAttribute('data-phase'),
        (phase) => phase === 'tests',
        'the request’s tests open on the line that failed',
      );
    } finally {
      server.close();
      await bridge.deleteHttpItem({ kind: 'collection', id: api.id });
    }
  });

  /**
   * #495: a value of the answer, right-clicked in its tree, becomes a rule of the request; the
   * next send writes it into the active environment, and the next request is sent with it.
   */
  it('extracts a value of a response into a variable the next request is sent with', async () => {
    const heard: string[] = [];
    const server = createServer((request, response) => {
      heard.push(`${request.url} ${String(request.headers.authorization ?? '-')}`);
      response.writeHead(200, { 'Content-Type': 'application/json' });
      response.end(request.url === '/login' ? '{"access_token":"zq-e2e-token","expires_in":3600}' : '{}');
    });
    await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve));
    const { port } = server.address() as AddressInfo;
    const api = await bridge.createHttpCollection('Extraction');
    const staging = await bridge.createHttpEnvironment('Extraction e2e');
    const login = await bridge.createHttpRequest({
      collectionId: api.id,
      folderId: null,
      name: 'Login',
      kind: 'http',
      method: 'GET',
      document: { url: `http://127.0.0.1:${port}/login`, description: '' },
    });
    const me = await bridge.createHttpRequest({
      collectionId: api.id,
      folderId: null,
      name: 'Moi',
      kind: 'http',
      method: 'GET',
      document: {
        url: `http://127.0.0.1:${port}/me`,
        description: '',
        headers: [{ key: 'Authorization', value: 'Bearer {{accessToken}}' }],
      },
    });
    try {
      await openHttp();
      await $(`${testid('http-node')}[data-id="${login.id}"] ${testid('http-node-name')}`).click();
      await $(testid('http-request-editor')).waitForDisplayed({ timeout: 10_000 });
      await pick(staging.id);
      await press('Enter', ['Control']);
      await $(testid('http-response-status')).waitForDisplayed({ timeout: 10_000 });

      await pickSegment('http-response-view', 'tree');
      const line = `${testid('json-line')}[data-path="$.access_token"]`;
      await $(line).waitForDisplayed({ timeout: 10_000 });
      await browser.execute((selector: string) => {
        const target = document.querySelector(selector)!;
        const box = target.getBoundingClientRect();
        target.dispatchEvent(
          new MouseEvent('contextmenu', {
            bubbles: true,
            cancelable: true,
            clientX: box.left + 10,
            clientY: box.top + 5,
          }),
        );
      }, line);
      await $(testid('json-menu-extract')).click();
      await $(testid('http-extract-dialog')).waitForDisplayed({ timeout: 10_000 });
      expect(await $(testid('http-extract-name')).getValue()).toBe('accessToken');
      expect(await $(testid('http-extract-secret')).isSelected()).toBe(true);
      await $(testid('http-extract-submit')).click();
      await $(testid('http-extract-dialog')).waitForExist({ reverse: true, timeout: 10_000 });
      await press('s', ['Control']);
      await eventually(
        async () => (await bridge.httpRequest(login.id)).document.extractions,
        (rules) => rules?.[0]?.path === 'body.access_token' && rules[0].variable === 'accessToken',
        'the rule saved with the request',
      );

      // Applied from the next send on, into the active environment.
      await press('Enter', ['Control']);
      const filled = await eventually(
        async () =>
          (await bridge.httpEnvironments()).environments
            .find((held) => held.id === staging.id)
            ?.variables.find((row) => row.name === 'accessToken'),
        (row) => row?.value === 'zq-e2e-token' && row.kind === 'secret',
        'the token written into the environment, a secret',
      );
      expect(filled?.filledBy?.name).toBe('Login');

      await $(`${testid('http-node')}[data-id="${me.id}"] ${testid('http-node-name')}`).click();
      await eventually(
        () => $(testid('http-request-name')).getValue(),
        (name) => name === 'Moi',
        'the next request open',
      );
      await press('Enter', ['Control']);
      await eventually(
        () => Promise.resolve(heard.at(-1)),
        (said) => said === '/me Bearer zq-e2e-token',
        'the next request sent with the value',
      );
    } finally {
      server.close();
      await bridge.deleteHttpItem({ kind: 'collection', id: api.id });
      await bridge.deleteHttpEnvironment(staging.id);
    }
  });

  /**
   * #497: the console under the answer, folded at first, says what left and came back and
   * what a script logged — a secret in force written as its name, an auth header masked.
   */
  it('shows the traffic and a script’s log in the console, its secret masked', async () => {
    const server = createServer((_request, response) => {
      response.writeHead(200, { 'Content-Type': 'application/json' });
      response.end('{}');
    });
    await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve));
    const { port } = server.address() as AddressInfo;
    const api = await bridge.createHttpCollection('Console');
    const secrets = await bridge.createHttpEnvironment('Console e2e');
    await bridge.saveHttpEnvironment(secrets.id, {
      variables: [{ name: 'token', value: 'zq-console-secret', kind: 'secret' }],
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
        headers: [{ key: 'Authorization', value: 'Bearer {{token}}' }],
        scripts: { before: "console.log('jeton', env.get('token'));" },
      },
    });
    try {
      await openHttp();
      await $(`${testid('http-node')}[data-id="${created.id}"] ${testid('http-node-name')}`).click();
      await $(testid('http-request-editor')).waitForDisplayed({ timeout: 10_000 });
      await pick(secrets.id);
      await press('Enter', ['Control']);
      await $(testid('http-response-status')).waitForDisplayed({ timeout: 10_000 });

      await eventually(
        () => $(testid('http-console-summary')).getText(),
        (summary) => summary.startsWith('1 '),
        'the folded console counting the exchange',
      );
      await $(testid('http-console-unfold')).click();
      await $(testid('http-console')).waitForDisplayed({ timeout: 10_000 });
      const said = await browser.execute(
        (selector: string) => [...document.querySelectorAll(selector)].map((line) => line.textContent ?? ''),
        `${testid('http-console-line')}, ${testid('http-console-sub')}`,
      );
      const all = said.join(' | ');
      expect(all).toContain('jeton {{token}}');
      expect(all).toContain('Authorization: Bearer ••••••••');
      expect(all).not.toContain('zq-console-secret');
      await $(testid('http-console-fold')).click();
      await $(testid('http-console-unfold')).waitForDisplayed({ timeout: 10_000 });
    } finally {
      server.close();
      await bridge.deleteHttpItem({ kind: 'collection', id: api.id });
      await bridge.deleteHttpEnvironment(secrets.id);
    }
  });
});
