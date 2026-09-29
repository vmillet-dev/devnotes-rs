import { $, $$, browser, expect } from '@wdio/globals';
import { mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { canvas } from '../pageobjects/canvas.page.js';
import {
  activeTestId,
  eventually,
  pickChoice,
  press,
  readEach,
  reloadCanvas,
  setField,
  testid,
} from '../support/app.js';
import { bridge, query } from '../support/bridge.js';

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
  let spaceId = '';

  before(async () => {
    await canvas.open();
    // A space of its own: the first launch's went with the library 24-forgotten-passphrase set aside.
    spaceId = (await bridge.createSpace({ name: 'Outils' })).id;
    await reloadCanvas();
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

  describe('the encoders', () => {
    it('encodes a URL component as it is typed, and decodes the other way', async () => {
      await openTool('url-codec');

      await setField(testid('url-codec-decoded'), 'café crème & co');
      await eventually(
        () => $(testid('url-codec-encoded')).getValue(),
        (value) => value === 'caf%C3%A9%20cr%C3%A8me%20%26%20co',
        'the encoding',
      );

      await setField(testid('url-codec-encoded'), '50%2');
      await $(testid('url-codec-problem')).waitForDisplayed({ timeout: 5_000 });
    });

    it('writes Base64 in either alphabet, padded or not', async () => {
      await openTool('base64');
      await setField(testid('base64-input'), 'été?>');
      await eventually(
        () => $(testid('base64-output')).getText(),
        (text) => text === 'w6l0w6k/Pg==',
        'standard',
      );

      await $(`${testid('segmented-base64-alphabet')} [data-segment-id="urlSafe"]`).click();
      await $(testid('base64-padded')).click();

      await eventually(
        () => $(testid('base64-output')).getText(),
        (text) => text === 'w6l0w6k_Pg',
        'URL-safe',
      );
    });

    it('names the character a decoding stops at', async () => {
      await $(`${testid('segmented-base64-direction')} [data-segment-id="decode"]`).click();
      await setField(testid('base64-input'), 'QUJD#');

      await $(testid('base64-problem')).waitForDisplayed({ timeout: 5_000 });
      expect(await $(testid('base64-problem')).getText()).toContain('#');
      await $(testid('tool-clear')).click();
      await $(`${testid('segmented-base64-direction')} [data-segment-id="encode"]`).click();
    });

    it('reads a file where it lies, by its path', async () => {
      const folder = mkdtempSync(join(tmpdir(), 'devnotes-e2e-'));
      const path = join(folder, 'logo.bin');
      writeFileSync(path, Buffer.from([0, 255, 16]));

      expect(await bridge.encodeBase64File(path, { alphabet: 'standard', padded: true })).toEqual({
        kind: 'encoded',
        name: 'logo.bin',
        bytes: 3,
        text: 'AP8Q',
      });
      expect(
        await bridge.encodeBase64File(join(folder, 'absent.bin'), { alphabet: 'standard', padded: true }),
      ).toEqual({
        kind: 'failed',
        problem: 'notFound',
      });
    });

    it('writes a colour in every notation', async () => {
      await openTool('colour');

      await setField(testid('colour-input'), 'hsl(0 100% 50%)');

      await eventually(outputValues, (values) => values[0] === '#ff0000', 'the hex');
      expect(await outputValues()).toEqual([
        '#ff0000',
        'rgb(255 0 0)',
        'hsl(0 100% 50%)',
        'oklch(62.8% 0.258 29.2)',
      ]);
    });

    it('measures a contrast as the palette test does', async () => {
      await setField(testid('colour-input'), '#777777');
      await setField(testid('colour-against'), '#ffffff');

      await eventually(
        () => $(testid('colour-ratio')).getText(),
        (ratio) => ratio === '4.48:1',
        'the ratio',
      );
      expect(await readEach(`${testid('colour-verdicts')} td`, '@class')).toEqual(['', 'pass', '', '']);
    });

    it('brings a colour sRGB cannot show inside, and says so', async () => {
      await setField(testid('colour-input'), 'oklch(70% 0.4 150)');

      await $(testid('colour-gamut')).waitForDisplayed({ timeout: 5_000 });
    });
  });

  describe('the crypto tools', () => {
    const FOX_HMAC_SHA256 = 'f7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8';

    it('keys a digest with a masked key, as the reference says', async () => {
      await openTool('hash');
      await setField(testid('hash-input'), 'The quick brown fox jumps over the lazy dog');
      await $(testid('hash-hmac')).click();
      await setField(testid('hash-key'), 'key');

      expect(await $(testid('hash-key')).getAttribute('type')).toBe('password');
      await eventually(
        () => $(`${testid('output-row')}[data-name="HMAC-SHA256"] ${testid('output-value')}`).getText(),
        (value) => value === FOX_HMAC_SHA256,
        'the HMAC',
      );
    });

    it('recognises a pasted signature, whatever its case', async () => {
      await setField(testid('hash-expected'), FOX_HMAC_SHA256.toUpperCase());

      await eventually(
        () => $(testid('hash-verdict')).getText(),
        (verdict) => verdict.includes('HMAC-SHA256') && verdict.includes('hex'),
        'the verdict',
      );
    });

    it('never gives the key back after a trip away', async () => {
      await $(testid('tool-back')).click();
      await entry('hash').click();

      await $(testid('hash-key')).waitForDisplayed({ timeout: 5_000 });
      expect(await $(testid('hash-key')).getValue()).toBe('');
      expect(await $(testid('hash-input')).getValue()).toBe('The quick brown fox jumps over the lazy dog');
      await $(testid('hash-hmac')).click();
    });

    it('draws passwords of every set chosen, and draws new ones on request', async () => {
      await openTool('password');

      await eventually(outputValues, (values) => values.length === 5, 'five passwords');
      const first = await outputValues();
      expect(first.every((password) => password.length === 20)).toBe(true);
      expect(
        first.every((password) => /[a-z]/.test(password) && /[A-Z]/.test(password) && /\d/.test(password)),
      ).toBe(true);

      await $(testid('password-generate')).click();
      await eventually(outputValues, (values) => values.length === 5 && values[0] !== first[0], 'a new draw');
    });

    it('warns before a password is kept as a note', async () => {
      await $(testid('tool-save-as-note')).click();

      await $(testid('save-as-note-warning')).waitForDisplayed({ timeout: 5_000 });
      await $(testid('save-as-note-cancel')).click();
      await $(testid('save-as-note')).waitForExist({ reverse: true, timeout: 5_000 });
    });

    it('draws v7 UUIDs that sort by creation, and reads the time back out of one', async () => {
      await openTool('uuid');
      await $(`${testid('segmented-uuid-version')} [data-segment-id="v7"]`).click();

      await eventually(
        async () => (await $(testid('uuid-list')).getText()).split('\n'),
        (lines) => lines.length === 5 && lines.every((line) => /^[0-9a-f]{8}-[0-9a-f]{4}-7/.test(line)),
        'five v7s',
      );
      const lines = (await $(testid('uuid-list')).getText()).split('\n');
      expect([...lines].sort()).toEqual(lines);

      await setField(testid('uuid-checked'), '01922b6e-4b30-7cc4-9a5c-6f2d8e1b3a77');
      await eventually(
        () => $(testid('uuid-created')).getText(),
        (created) => created === '2024-09-25 23:05:01.488 UTC',
        'its time',
      );
    });
  });

  describe('a result kept as a note', () => {
    const inSpace = async () =>
      (await bridge.queryNotes(query({ spaceId }))).sections.flatMap((section) => section.notes);

    after(async () => {
      const ids = (await inSpace()).map((note) => note.id);
      await bridge.deleteNotes(ids);
      await bridge.purgeNotes(ids);
    });

    it('asks nothing before the tool has computed something', async () => {
      await openTool('slug');
      await $(testid('tool-clear')).click();

      await eventually(
        () => $(testid('tool-save-as-note')).getAttribute('aria-disabled'),
        (disabled) => disabled === 'true',
        'nothing to keep',
      );
    });

    it('goes where the dialog puts it, with its tags, as the tool shaped it', async () => {
      await setField(testid('slug-input'), 'Été 2026 !');
      await eventually(outputValues, (values) => values[0] === 'ete-2026', 'the slug');

      await $(testid('tool-save-as-note')).click();
      await $(testid('save-as-note')).waitForDisplayed({ timeout: 5_000 });
      await setField(testid('save-as-note-title'), 'Slug de la page été');
      await pickChoice('save-as-note-space', spaceId);
      await setField(testid('save-as-note-tags'), 'web, seo');
      await $(testid('save-as-note-submit')).click();
      await $(testid('save-as-note')).waitForExist({ reverse: true, timeout: 10_000 });

      const [note] = await inSpace();
      expect(note).toMatchObject({
        title: 'Slug de la page été',
        content: 'ete-2026',
        language: 'txt',
        kind: 'snippet',
      });
      expect([...(note?.tags ?? [])].sort()).toEqual(['seo', 'web']);
      expect(note?.source).toContain('/');
    });
  });
});
