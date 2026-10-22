import { mkdtempSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:http';
import type { AddressInfo } from 'node:net';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { $, expect } from '@wdio/globals';

import { canvas } from '../pageobjects/canvas.page.js';
import { eventually, press, readEach, testid } from '../support/app.js';
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

describe('HTTP runner', () => {
  before(async () => {
    await canvas.open();
  });

  after(async () => {
    await press('1', ['Control']);
    await $(testid('http-page')).waitForExist({ reverse: true, timeout: 10_000 });
  });

  /**
   * #496: a collection of two requests run twice — four sends, each with its iteration —, its
   * summary, the request that failed open on its assertion, and nothing in the history. The OS
   * picker cannot be answered (the picker note in `support/app.ts`): the data file is read through
   * its command, its control asserted on.
   */
  it('runs a collection over its iterations and sums the run up', async () => {
    const heard: string[] = [];
    const server = createServer((request, response) => {
      heard.push(request.url ?? '');
      const missing = request.url === '/clients/cus_2';
      response.writeHead(missing ? 404 : 200, { 'Content-Type': 'application/json' });
      response.end('{}');
    });
    await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve));
    const { port } = server.address() as AddressInfo;
    const directory = mkdtempSync(join(tmpdir(), 'devnotes-e2e-'));
    const csv = join(directory, 'clients.csv').replaceAll('\\', '/');
    writeFileSync(csv, 'customer\ncus_1\ncus_2\n');
    const prose = join(directory, 'runner-invalide.txt').replaceAll('\\', '/');
    writeFileSync(prose, 'ceci n’est ni un CSV avec en-tête ni un tableau JSON\n');
    const historyBefore = (await bridge.httpHistory(0)).flatMap((day) => day.items).length;

    const api = await bridge.createHttpCollection('Exécution e2e');
    for (const [name, path] of [
      ['Login', '/login?c={{customer}}'],
      ['Un client', '/clients/{{customer}}'],
    ]) {
      await bridge.createHttpRequest({
        collectionId: api.id,
        folderId: null,
        name: name!,
        kind: 'http',
        method: 'GET',
        document: {
          url: `http://127.0.0.1:${port}${path}`,
          description: '',
          scripts: {
            before: "pm.variables.set('customer', 'cus_' + (pm.info.iteration + 1));",
            tests: "test('trouvé', () => expect(res.status).toBe(200));",
          },
        },
      });
    }
    try {
      await openHttp();
      await $(`${testid('http-node-menu')}[data-id="${api.id}"]`).click();
      await $(`${testid('http-node-action')}[data-action="run"]`).click();
      await $(testid('http-runner')).waitForDisplayed({ timeout: 10_000 });
      expect(await $(testid('http-runner-title')).getText()).toBe('Exécution e2e');
      expect(await readEach(testid('http-runner-row'), '@aria-label')).toEqual(['Login', 'Un client']);

      expect(await $(testid('http-runner-data-choose')).isExisting()).toBe(true);
      expect(await bridge.readHttpRunData(csv)).toEqual({
        columns: ['customer'],
        rows: [{ customer: 'cus_1' }, { customer: 'cus_2' }],
      });
      // One line of text is no CSV header: it is neither, not a file without rows (HSC-17).
      await expect(bridge.readHttpRunData(prose)).rejects.toThrow('"fault":"unreadable"');
      await $(testid('http-runner-iterations')).setValue('2');
      await press('Tab');

      // Read without its words: the runner's language is the machine's.
      await $(testid('http-runner-start')).click();
      await eventually(
        () => $(testid('http-runner-figures')).getText(),
        (figures) => /^4 \S+ · 3\/4 /.test(figures.replace(/\s+/g, ' ').trim()),
        'the run summed up',
      );
      expect((await $(testid('http-runner-summary')).getText()).trim()).toMatch(/^1 /);
      expect(heard).toEqual(['/login?c=cus_1', '/clients/cus_1', '/login?c=cus_2', '/clients/cus_2']);
      expect(
        (await readEach(testid('http-runner-iteration'), 'text')).map((header) => header.replace(/^\D+/, '')),
      ).toEqual(['1 / 2', '2 / 2']);
      expect(await readEach(testid('http-runner-result'), '@data-result')).toEqual([
        'passed',
        'passed',
        'passed',
        'failed',
      ]);
      expect(await $(testid('http-runner-details')).getText()).toContain('trouvé');

      await $(`${testid('http-runner-filter')}[data-filter="failed"]`).click();
      expect(await readEach(testid('http-runner-result'), '@data-result')).toEqual(['failed']);

      // A run writes no history: its report is the session's.
      expect((await bridge.httpHistory(0)).flatMap((day) => day.items).length).toBe(historyBefore);
      expect(await $(testid('error-banner')).isExisting()).toBe(false);
    } finally {
      server.close();
      await bridge.deleteHttpItem({ kind: 'collection', id: api.id });
    }
  });
});
