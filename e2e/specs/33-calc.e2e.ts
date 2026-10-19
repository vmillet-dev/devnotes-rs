import { $, $$, browser, expect } from '@wdio/globals';

import { canvas } from '../pageobjects/canvas.page.js';
import { editor } from '../pageobjects/editor.page.js';
import { spaces } from '../pageobjects/sidebar.page.js';
import { clipboardText, eventually, readEach, reloadCanvas, testid } from '../support/app.js';
import { bridge } from '../support/bridge.js';

/**
 * A label, a scale and the previous result, a variable, a total of its block, a conversion, a
 * percentage, a refused currency and a word of the grammar refused as a name: what a sheet is
 * for, in the words of both languages. The results are read by their figures and classes alone
 * — the language on screen writes the decimal mark and the units.
 */
const SHEET = [
  'Annuel : 1,5k × 2',
  'Mensuel : préc / 12',
  '# Budget',
  'hébergement = 100,521 €',
  'domaines = 4,99 €',
  'mensuel = total',
  'mensuel × 12',
  '3 h 20 min en min',
  '20 % of 340',
  '100 € en $',
  'en = 5',
].join('\n');

const digits = (text: string): string => text.replace(/\D/g, '');

/** Into CodeMirror as a keyboard would put it: its `.cm-line` also holds the results. */
async function typeSheet(text: string): Promise<void> {
  await browser.execute(
    (selector: string, value: string) => {
      (document.querySelector(selector) as HTMLElement).focus();
      document.execCommand('selectAll');
      document.execCommand('insertText', false, value);
    },
    `${testid('editor-body')} .cm-content`,
    text,
  );
}

describe('A Calcul note', () => {
  let spaceId = '';

  before(async () => {
    await canvas.open();
    // A space of its own, chosen: the new note lands in the one on screen.
    spaceId = (await bridge.createSpace({ name: 'Calcul' })).id;
    await reloadCanvas();
    await spaces.open();
    await spaces.option(spaceId).click();
  });

  it('is created from the new-note menu, a fourth kind beside the others', async () => {
    await $(testid('new-note-kind')).click();
    await $(testid('new-note-calc')).waitForDisplayed({ timeout: 10_000 });
    await $(testid('new-note-calc')).click();

    await editor.waitOpen();
    await $(`${testid('editor-body')}[data-kind="calc"]`).waitForExist({ timeout: 10_000 });
    await editor.setTitle('Budget de recette');
  });

  it('computes each line as it is typed, and an error stays on its line', async () => {
    await typeSheet(SHEET);

    await eventually(
      async () => (await readEach(testid('calc-result'), 'text')).map(digits),
      (results) => results.join(' ') === '3000 250 10052 499 10551 126613 200 68  ',
      'one result a line, the heading without one',
    );
    const failed = await readEach(testid('calc-result'), '@class');
    expect(failed.map((classes) => classes.includes('calc-failed'))).toEqual([
      false,
      false,
      false,
      false,
      false,
      false,
      false,
      false,
      true,
      true,
    ]);
    expect((await readEach(testid('calc-result'), '@data-error')).slice(-2)).toEqual([
      'currency',
      'reserved',
    ]);
    expect(digits(await $(`${testid('calc-footer')} .counts`).getText())).toBe('113');
  });

  it('adds up the result picked in its footer', async () => {
    const picks = await $$(`${testid('calc-result')} [data-pick]`).getElements();
    await picks[3]!.click();

    await eventually(
      async () => digits(await $(testid('calc-sum')).getText()),
      (sum) => sum === '499',
      'the sum of the result picked',
    );
    await editor.close();
  });

  it('shows on its card the last lines that gave something, the total under a rule', async () => {
    const card = await canvas.cardWithTitle('Budget de recette');
    const lines = await card.$$(testid('note-card-calc-line')).getElements();

    expect(lines).toHaveLength(4);
    expect(await lines[0]!.getAttribute('class')).toContain('total');
    expect(digits(await lines[1]!.getText())).toContain('126613');
  });

  it('copies with its results as comments, which read back as the same sheet', async function () {
    const card = await canvas.cardWithTitle('Budget de recette');
    await card.moveTo();
    await card.$(testid('copy-button')).click();

    // An unreadable clipboard answers null at once, so only the readable case waits.
    const copied = await eventually(
      () => clipboardText(),
      (text) => text === null || text.includes('hébergement = 100,521 € //'),
      'the copy to reach the clipboard',
    );
    if (copied === null) {
      // No readable clipboard on this runner; see `clipboardText`.
      this.skip();
      return;
    }
    expect(copied.split(/\r?\n/)[3]).toMatch(/^hébergement = 100,521 € \/\/ ≈ /);

    const today = new Date().toISOString().slice(0, 10);
    const [again, original] = await Promise.all([
      bridge.evaluateCalc(copied, today),
      bridge.evaluateCalc(SHEET, today),
    ]);
    expect(again.rows.map((row) => row.line)).toEqual(original.rows.map((row) => row.line));
  });
});
