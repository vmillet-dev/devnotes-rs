import { $, $$, browser, expect } from '@wdio/globals';

import { canvas } from '../pageobjects/canvas.page.js';
import { activeTestId, eventually, press, readEach, setField, testid } from '../support/app.js';
import { bridge } from '../support/bridge.js';

const entry = (tool: string) => $(`${testid('tools-entry')}[data-tool="${tool}"]`);
const outputValues = () => readEach(testid('output-value'), 'text');

async function openTool(tool: string): Promise<void> {
  await $(testid('tools-rail-all')).click();
  await entry(tool).click();
  await eventually(
    () => $(testid('tool-body')).getAttribute('data-tool'),
    (shown) => shown === tool,
    `${tool} to open`,
  );
}

/** A paste as the system makes it, CRs included: a textarea's value would lose them. */
async function pasteInto(selector: string, text: string): Promise<void> {
  await browser.execute(
    (target: string, pasted: string) => {
      const data = new DataTransfer();
      data.setData('text/plain', pasted);
      document
        .querySelector(target)!
        .dispatchEvent(new ClipboardEvent('paste', { clipboardData: data, bubbles: true, cancelable: true }));
    },
    selector,
    text,
  );
}

describe('The tools', () => {
  before(async () => {
    await canvas.open();
  });

  after(async () => {
    await press('1', ['Control']);
    await $(testid('tools-page')).waitForExist({ reverse: true, timeout: 10_000 });
  });

  describe('their home', () => {
    it('opens on its search from the notes, on Ctrl+Shift+T', async () => {
      await press('T', ['Control', 'Shift']);

      await $(testid('tools-page')).waitForDisplayed({ timeout: 10_000 });
      await eventually(activeTestId, (id) => id === 'tools-search', 'the search field to take the focus');
    });

    it('says what it holds reaches no network', async () => {
      expect(await $(testid('tools-promise')).isDisplayed()).toBe(true);
    });

    it('finds a tool by one of its keywords, in any case', async () => {
      await setField(testid('tools-search'), 'CrLf');

      await eventually(
        () => readEach(testid('tools-entry'), '@data-tool'),
        (tools) => tools.join() === 'line-breaks',
        'the search to narrow',
      );
    });

    it('says so when no tool answers a search', async () => {
      await setField(testid('tools-search'), 'zzz-no-such-tool');

      await $(testid('tools-no-match')).waitForDisplayed({ timeout: 5_000 });
      await setField(testid('tools-search'), '');
    });

    it('lays the Texte panel out with its four tools', async () => {
      expect(
        await readEach(
          `${testid('tools-panel')}[data-category="text"] ${testid('tools-entry')}`,
          '@data-tool',
        ),
      ).toEqual(['case', 'slug', 'url-parser', 'line-breaks']);
    });
  });

  describe('the text tools', () => {
    it('converts a text into every case', async () => {
      await openTool('case');

      await setField(testid('case-input'), 'parse HTTP response');

      await eventually(outputValues, (values) => values.length === 7, 'the seven cases');
      expect(await outputValues()).toEqual([
        'parseHttpResponse',
        'ParseHttpResponse',
        'parse_http_response',
        'parse-http-response',
        'PARSE_HTTP_RESPONSE',
        'Parse Http Response',
        'Parse http response',
      ]);
    });

    it('keeps what was typed across a trip to the notes', async () => {
      await press('1', ['Control']);
      await $(testid('tools-page')).waitForExist({ reverse: true, timeout: 10_000 });
      await press('2', ['Control']);

      await $(testid('case-input')).waitForDisplayed({ timeout: 10_000 });
      expect(await $(testid('case-input')).getValue()).toBe('parse HTTP response');
    });

    it('empties on Vider', async () => {
      await $(testid('tool-clear')).click();

      await eventually(
        () => $(testid('case-input')).getValue(),
        (value) => value === '',
        'the field to empty',
      );
      await eventually(outputValues, (values) => values.length === 0, 'the cases to go');
    });

    it('transliterates a slug rather than dropping its accents', async () => {
      await $(`${testid('tools-rail-tool')}[data-tool="slug"]`).click();

      await setField(testid('slug-input'), 'Été 2026 ! Straße');

      await eventually(outputValues, (values) => values[0] === 'ete-2026-strasse', 'the slug');
    });

    it('takes a URL apart, and says why another is refused', async () => {
      await $(`${testid('tools-rail-tool')}[data-tool="url-parser"]`).click();

      await setField(testid('url-parser-input'), 'https://api.exemple.fr:8443/v1?statut=pay%C3%A9e#bas');
      await $(testid('url-parser-parameters')).waitForDisplayed({ timeout: 5_000 });
      expect(await $(testid('url-parser-parameters')).getText()).toContain('payée');
      expect(await $('[data-part="port"]').getText()).toContain('8443');

      await setField(testid('url-parser-input'), 'exemple.fr/chemin');
      await $(testid('url-parser-problem')).waitForDisplayed({ timeout: 5_000 });
    });

    it('answers a mixture of line endings from Rust, counted by kind', async () => {
      const answer = await bridge.fixLineBreaks({
        text: 'a  \r\nb\nc',
        ending: 'crlf',
        trimTrailing: true,
        finalNewline: 'add',
      });

      expect(answer).toEqual({
        found: { lf: 1, crlf: 1, cr: 0 },
        text: 'a\r\nb\r\nc\r\n',
        converted: 1,
        trimmed: 1,
        finalNewline: 'added',
      });
    });

    it('shows the endings of a pasted text, and gives them back as one kind', async () => {
      await $(`${testid('tools-rail-tool')}[data-tool="line-breaks"]`).click();
      await $(testid('line-breaks-input')).waitForDisplayed({ timeout: 10_000 });

      await pasteInto(testid('line-breaks-input'), 'a\r\nb\nc\n');
      await $(testid('line-breaks-mixed')).waitForDisplayed({ timeout: 5_000 });

      await $(`${testid('segmented-line-breaks-ending')} [data-segment-id="lf"]`).click();
      await eventually(
        () => $$(`${testid('line-breaks-output')} [data-ending="crlf"]`).length,
        (count) => count === 0,
        'the output to hold LF alone',
      );
      expect(await $(testid('line-breaks-changes')).getText()).toContain('1');
    });

    it('lists the tools opened last on the home, most recent first', async () => {
      await $(testid('tool-back')).click();

      await eventually(
        () => readEach(testid('tools-recent'), '@data-tool'),
        (tools) => tools.slice(0, 3).join() === 'line-breaks,url-parser,slug',
        'the recent tools',
      );
    });
  });
});
