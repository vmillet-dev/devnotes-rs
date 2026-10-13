import { $, expect } from '@wdio/globals';

import { canvas } from '../pageobjects/canvas.page.js';
import { eventually, pickChoice, press, readEach, reloadCanvas, testid } from '../support/app.js';
import { bridge, query } from '../support/bridge.js';

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

describe('HTTP code generation', () => {
  // Past `24-forgotten-passphrase` the home space is gone: this one is where the snippet goes,
  // made before the page reads the spaces.
  let spaceId = '';

  before(async () => {
    await canvas.open();
    spaceId = (await bridge.createSpace({ name: 'Code généré' })).id;
    await reloadCanvas();
  });

  after(async () => {
    await press('1', ['Control']);
    await $(testid('http-page')).waitForExist({ reverse: true, timeout: 10_000 });
  });

  /**
   * #488: the request written for cURL and the shells, from what a send would write — the
   * environment's values in it, its secret left as `{{token}}` until it is asked for in clear —
   * then kept as a snippet tagged `http`, in its target's language.
   */
  it('writes the request for cURL and PowerShell, its secret left as a variable, and keeps it as a snippet', async () => {
    const api = await bridge.createHttpCollection('Code');
    const staging = await bridge.createHttpEnvironment('Staging code');
    await bridge.saveHttpEnvironment(staging.id, {
      variables: [
        { name: 'baseUrl', value: 'https://api.exemple.fr' },
        { name: 'token', value: 'zq-code-secret', kind: 'secret' },
      ],
    });
    const created = await bridge.createHttpRequest({
      collectionId: api.id,
      folderId: null,
      name: 'Factures',
      kind: 'http',
      method: 'GET',
      document: {
        url: '{{baseUrl}}/invoices?page=1',
        description: '',
        auth: { kind: 'bearer', token: '{{token}}' },
      },
    });
    try {
      await openHttp();
      await $(`${testid('http-node')}[data-id="${created.id}"] ${testid('http-node-name')}`).click();
      await $(testid('http-request-editor')).waitForDisplayed({ timeout: 10_000 });
      await $(testid('http-environment-picker')).click();
      await $(`${testid('http-environment-option')}[data-id="${staging.id}"]`).click();

      await $(testid('http-code-open')).click();
      await $(testid('http-code-dialog')).waitForDisplayed({ timeout: 10_000 });
      await $(`${testid('http-code-target')}[data-target="curl"]`).click();
      const code = () => $(testid('http-code-output')).getText();
      await eventually(
        code,
        (text) => text.includes("--url 'https://api.exemple.fr/invoices?page=1'"),
        'the URL resolved with Staging',
      );
      expect(await code()).toContain('Bearer {{token}}');
      expect(await code()).not.toContain('zq-code-secret');
      expect(await $(testid('http-code-note')).getAttribute('data-note')).toBe('secrets');

      await $(testid('http-code-secrets')).click();
      await $(testid('http-code-secrets-ask')).waitForDisplayed({ timeout: 10_000 });
      await $(testid('http-code-secrets-yes')).click();
      await eventually(code, (text) => text.includes('Bearer zq-code-secret'), 'the secret written in clear');
      await $(testid('http-code-secrets')).click();
      await eventually(code, (text) => text.includes('Bearer {{token}}'), 'the secret a variable again');

      await $(`${testid('http-code-target')}[data-target="powershell"]`).click();
      await eventually(code, (text) => text.includes('Invoke-RestMethod'), 'the PowerShell call');

      await $(testid('http-code-save')).click();
      await $(testid('save-as-note')).waitForDisplayed({ timeout: 10_000 });
      expect(await $(testid('save-as-note-tags')).getValue()).toBe('http');
      await pickChoice('save-as-note-space', spaceId);
      await $(testid('save-as-note-submit')).click();
      const saved = await eventually(
        async () =>
          (await bridge.queryNotes(query({ spaceId: spaceId }))).sections.flatMap((section) => section.notes),
        (notes) => notes.length === 1,
        'the snippet kept in the space',
      );
      expect(saved[0]?.language).toBe('ps1');
      expect(saved[0]?.tags).toContain('http');

      await $(testid('http-code-dialog')).waitForDisplayed({ timeout: 10_000 });
      await press('Escape');
      await $(testid('http-code-dialog')).waitForExist({ reverse: true, timeout: 10_000 });
      expect(await readEach(testid('http-tab'), '@data-key')).toContain(created.id);
      const tab = `${testid('http-tab')}[data-key="${created.id}"]`;
      await $(`${tab} ${testid('http-tab-close')}`).click();
      await $(tab).waitForExist({ reverse: true, timeout: 10_000 });
      await $(testid('http-environment-picker')).click();
      await $(`${testid('http-environment-option')}[data-id="none"]`).click();
      for (const note of saved) {
        await bridge.deleteNotes([note.id]);
        await bridge.purgeNotes([note.id]);
      }
    } finally {
      await bridge.deleteHttpEnvironment(staging.id);
      await bridge.deleteHttpItem({ kind: 'collection', id: api.id });
    }
  });
});
