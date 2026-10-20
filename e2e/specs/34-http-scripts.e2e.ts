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
});
