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
import { bridge, draft, query } from '../support/bridge.js';

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

    it('converts JSON to YAML in the order the document keeps, and back', async () => {
      await openTool('convert');
      await setField(testid('convert-input'), '{"service":"billing","replicas":2,"database":{"pool":10}}');

      await eventually(
        () => $(testid('convert-output')).getText(),
        (yaml) => yaml.startsWith('service: billing'),
        'the YAML',
      );

      await $(testid('tool-action-swap')).click();
      await eventually(
        () => $(testid('convert-output')).getText(),
        (json) => json.startsWith('{') && json.indexOf('service') < json.indexOf('replicas'),
        'the JSON back',
      );
    });

    it('says what TOML cannot hold, and where', async () => {
      await $(`${testid('segmented-convert-from')} [data-segment-id="json"]`).click();
      await $(`${testid('segmented-convert-to')} [data-segment-id="toml"]`).click();
      await setField(testid('convert-input'), '{"database":{"replica":null}}');

      await eventually(
        () => $(testid('convert-impossible')).getText(),
        (text) => text.includes('$.database.replica'),
        'the refusal',
      );
    });

    it('reads XML attributes and repeated elements under the stated convention', async () => {
      await $(`${testid('segmented-convert-from')} [data-segment-id="xml"]`).click();
      await $(`${testid('segmented-convert-to')} [data-segment-id="json"]`).click();
      await setField(testid('convert-input'), '<order id="42"><line>A</line><line>B</line></order>');

      await eventually(
        () => $(testid('convert-output')).getText(),
        (json) =>
          JSON.stringify(JSON.parse(json)) === JSON.stringify({ order: { '@id': '42', line: ['A', 'B'] } }),
        'the JSON',
      );
      expect(await $(testid('convert-convention-xml')).isDisplayed()).toBe(true);
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

  describe('the generators', () => {
    const output = () => $(testid('json-generator-output')).getText();

    it('draws the same documents again from the same seed', async () => {
      await openTool('json-generator');
      await $(`${testid('segmented-json-generator-source')} [data-segment-id="example"]`).click();
      await setField(testid('json-generator-seed'), '7');
      await setField(testid('json-generator-input'), '{"id":"9b2f6c1e-3d4a-4f8b-9e2c-7a1d5b6c8e90","n":3}');
      await eventually(output, (text) => text.startsWith('['), 'the documents');
      const first = await output();

      await setField(testid('json-generator-seed'), '8');
      await eventually(output, (text) => text !== first, 'other documents');
      await setField(testid('json-generator-seed'), '7');

      await eventually(output, (text) => text === first, 'the first documents again');
      expect((JSON.parse(first) as unknown[]).length).toBe(3);
    });

    it('lists the schema keywords it did not honour', async () => {
      await $(`${testid('segmented-json-generator-source')} [data-segment-id="schema"]`).click();
      await setField(
        testid('json-generator-input'),
        '{"type":"object","required":["code"],"properties":{"code":{"type":"string","pattern":"^[A-Z]{3}$"}}}',
      );

      await eventually(
        () => $(testid('json-generator-unsupported')).getText(),
        (text) => text.includes('pattern') && text.includes('$.properties.code'),
        'the unsupported keyword',
      );
    });

    it('opens Lorem ipsum on the classic words', async () => {
      await openTool('lorem');
      await $(`${testid('segmented-lorem-unit')} [data-segment-id="words"]`).click();
      await setField(testid('lorem-count'), '5');

      await eventually(
        () => $(testid('lorem-output')).getText(),
        (text) => text === 'Lorem ipsum dolor sit amet,',
        'the opening words',
      );
    });
  });

  describe('the time tools', () => {
    const form = (id: string) => $(`[data-form="${id}"] ${testid('output-value')}`).getText();

    it('reads the last second of a 32-bit clock, in Rust, whatever the zone', async () => {
      const answer = await bridge.describeInstant({ text: '2038-01-19T03:14:07Z', magnitude: null });

      expect(answer.kind).toBe('read');
      if (answer.kind !== 'read') return;
      expect(answer.forms.unixSeconds).toBe('2147483647');
      expect(answer.forms.weekDate).toMatch(/^2038-W03-/);
      expect(await bridge.describeInstant({ text: '-86400', magnitude: null })).toMatchObject({
        forms: { isoUtc: '1969-12-31T00:00:00Z' },
      });
    });

    it('sits in a Temps panel of its own', async () => {
      await $(testid('tools-rail-all')).click();

      expect(
        await readEach(
          `${testid('tools-panel')}[data-category="time"] ${testid('tools-entry')}`,
          '@data-tool',
        ),
      ).toEqual(['dates', 'zones', 'cron']);
    });

    it('reads a timestamp by its size, and in the unit forced', async () => {
      await openTool('dates');
      await setField(testid('dates-input'), '1790000000123');

      await eventually(
        () => form('isoUtc'),
        (iso) => iso === '2026-09-21T14:13:20.123Z',
        'the ISO form',
      );
      expect(await $(testid('dates-reading')).getAttribute('data-reading')).toBe('milliseconds');
      expect(await form('unixSeconds')).toBe('1790000000');

      await $(`${testid('segmented-dates-magnitude')} [data-segment-id="microseconds"]`).click();
      await eventually(
        () => form('isoUtc'),
        (iso) => iso === '1970-01-21T17:13:20.000123Z',
        'microseconds',
      );
    });

    it('says where a date stops making sense', async () => {
      await setField(testid('dates-input'), '2026-13-01');

      await eventually(
        () => $(testid('dates-problem')).getAttribute('data-problem'),
        (problem) => problem === 'unreadable',
        'the refusal',
      );
      expect(await $(testid('dates-problem')).getText()).toContain('6');
    });

    it('takes the present instant from Rust', async () => {
      await $(testid('dates-now')).click();

      await eventually(
        () => $(testid('dates-input')).getValue(),
        (text) => /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/.test(text),
        'the instant in the field',
      );
      await eventually(
        () => $(testid('dates-reading')).getAttribute('data-reading'),
        (reading) => reading === 'iso8601',
        'read as ISO 8601',
      );
      const shown = Date.parse(await $(testid('dates-input')).getValue());
      expect(Math.abs(Date.now() - shown)).toBeLessThan(60_000);
      await $(testid('tool-clear')).click();
    });

    const zoneRow = (zone: string) => `${testid('zones-row')}[data-zone="${zone}"]`;
    const zoneTime = (zone: string) => $(`${zoneRow(zone)} ${testid('zones-time')}`).getText();

    it('gives both readings of the hour Paris goes through twice, from Rust', async () => {
      const answer = await bridge.placeInZones({
        text: '2026-10-25 02:30',
        from: 'Europe/Paris',
        zones: ['Asia/Kolkata'],
        later: false,
      });

      expect(answer).toMatchObject({
        kind: 'placed',
        ambiguous: [{ isoUtc: '2026-10-25T00:30:00Z' }, { isoUtc: '2026-10-25T01:30:00Z' }],
      });
      expect((await bridge.searchTimeZones('kathmandu')).map((entry) => entry.zone)).toEqual([
        'Asia/Kathmandu',
      ]);
    });

    it('places an instant in every zone of the list, a quarter-hour zone added', async () => {
      await openTool('zones');
      await setField(testid('zones-search'), 'kathmandu');
      await $(`${testid('zones-found-entry')}[data-zone="Asia/Kathmandu"]`).click();
      await setField(testid('zones-input'), '2026-09-29T12:00:00Z');

      await eventually(
        () => zoneTime('Asia/Kathmandu'),
        (time) => time === '17:45:00',
        'Kathmandu',
      );
      expect(await zoneTime('UTC')).toBe('12:00:00');
      expect(await zoneTime('Asia/Tokyo')).toBe('21:00:00');
      expect(await zoneTime('America/New_York')).toBe('08:00:00');
    });

    it('reads a time typed in another zone of the list, and names the day it is elsewhere', async () => {
      await $(`${zoneRow('Asia/Tokyo')} ${testid('zones-type-in')}`).click();
      await setField(testid('zones-input'), '2026-09-30 03:30');

      await eventually(
        () => $(testid('zones-from')).getAttribute('data-zone'),
        (zone) => zone === 'Asia/Tokyo',
        'typed in Tokyo',
      );
      await eventually(
        () => zoneTime('UTC'),
        (time) => time === '18:30:00',
        'the evening before in UTC',
      );
      expect(await $(`${zoneRow('UTC')} ${testid('zones-shift')}`).isDisplayed()).toBe(true);
    });

    it('says an hour New York skips does not exist', async () => {
      await $(`${zoneRow('America/New_York')} ${testid('zones-type-in')}`).click();
      await setField(testid('zones-input'), '2026-03-08 02:30');

      await eventually(
        () => $(testid('zones-problem')).getAttribute('data-problem'),
        (problem) => problem === 'skipped',
        'the skipped hour',
      );
      expect(await $(testid('zones-problem')).getText()).toContain('UTC-05:00');
      await $(testid('tool-clear')).click();
    });
  });

  describe('cron', () => {
    it('takes an expression apart, checks a date and names the field it refuses, in Rust', async () => {
      const answer = await bridge.describeCron({
        expression: '*/15 9-18 * * MON-FRI',
        zone: 'UTC',
        check: '2026-09-29 09:15',
      });

      expect(answer).toMatchObject({
        kind: 'read',
        bothDays: false,
        check: { kind: 'checked', matches: true },
      });
      if (answer.kind !== 'read') return;
      expect(answer.fields.map((field) => field.field)).toEqual([
        'minute',
        'hour',
        'dayOfMonth',
        'month',
        'dayOfWeek',
      ]);
      expect(answer.runs).toHaveLength(10);
      expect(await bridge.describeCron({ expression: '61 * * * *', zone: 'UTC', check: '' })).toEqual({
        kind: 'refused',
        field: 'minute',
        token: '61',
        at: 1,
        problem: 'outOfRange',
      });
    });

    it('reads an expression aloud with its next runs, in the zone chosen', async () => {
      await openTool('cron');
      await setField(testid('cron-zone-search'), 'utc');
      await $(`${testid('cron-zone-found')}[data-zone="UTC"]`).click();
      await setField(testid('cron-input'), '0 12 * * *');

      await eventually(
        () => readEach(testid('cron-run-at'), 'text'),
        (runs) => runs.length === 10 && runs.every((run) => run.endsWith('12:00:00')),
        'ten runs at noon',
      );
      expect(await $(testid('cron-zone')).getText()).toBe('UTC');
      expect(await $(testid('cron-sentence')).getText()).toContain('12:00');
    });

    it('says whether a date matches', async () => {
      await setField(testid('cron-input'), '0 9 * * 1');
      await setField(testid('cron-check'), '2026-09-28 09:00');

      await eventually(
        () => $(testid('cron-verdict')).getAttribute('data-verdict'),
        (verdict) => verdict === 'true',
        'a Monday at nine',
      );
      await setField(testid('cron-check'), '2026-09-29 09:00');
      await eventually(
        () => $(testid('cron-verdict')).getAttribute('data-verdict'),
        (verdict) => verdict === 'false',
        'a Tuesday',
      );
    });

    it('names the field of a refusal, and says what @reboot means', async () => {
      await setField(testid('cron-input'), '* 25 * * *');
      await eventually(
        () => $(testid('cron-problem')).getAttribute('data-field'),
        (field) => field === 'hour',
        'the hour refused',
      );

      await setField(testid('cron-input'), '@reboot');
      await $(testid('cron-reboot')).waitForDisplayed({ timeout: 5_000 });
      await $(testid('tool-clear')).click();
    });
  });

  describe('the calculators', () => {
    const digits = (text: string) => text.replace(/\D/g, '');
    const sizeValue = (unit: string) => $(`[data-unit="${unit}"] ${testid('output-value')}`).getText();

    it('counts a gigabyte as a billion bytes exactly, in Rust', async () => {
      const answer = await bridge.convertSize({ text: '1 GB', unit: 'byte', decimals: 3 });

      expect(answer.kind).toBe('converted');
      if (answer.kind !== 'converted') return;
      const exact = Object.fromEntries(answer.rows.map((row) => [row.unit, row.exact]));
      expect(exact['byte']).toBe('1000000000');
      expect(exact['gibibyte']).toBe('0.931322574615478515625');
      expect(await bridge.convertSize({ text: '-5 MB', unit: 'byte', decimals: 3 })).toEqual({
        kind: 'negative',
        at: 1,
      });
    });

    it('sits in a Calcul panel of its own', async () => {
      await $(testid('tools-rail-all')).click();

      expect(
        await readEach(
          `${testid('tools-panel')}[data-category="calc"] ${testid('tools-entry')}`,
          '@data-tool',
        ),
      ).toEqual(['sizes', 'percentages', 'permissions']);
    });

    it('writes a quantity typed in French symbols in every unit, rounded as asked', async () => {
      await openTool('sizes');
      await setField(testid('sizes-input'), '1,5 Go');

      await eventually(
        async () => digits(await sizeValue('byte')),
        (bytes) => bytes === '1500000000',
        'the bytes',
      );
      expect(await $(testid('sizes-reading')).getAttribute('data-unit')).toBe('gigabyte');
      expect(digits(await $(testid('sizes-gap')).getText())).toBe('10931');

      await setField(testid('sizes-decimals'), '6');
      await eventually(
        async () => digits(await sizeValue('gibibyte')),
        (gibibytes) => gibibytes === '1396984',
        'six decimals',
      );
    });

    it('answers percentages exactly, and says a division by zero, in Rust', async () => {
      const pair = (x: string, y: string) => ({ x, y });
      const answer = await bridge.answerPercentages({
        of: pair('1,1', '100'),
        share: pair('36', '0'),
        change: pair('0', '100'),
        apply: pair('15', '240'),
        lower: true,
        before: pair('15', '276'),
        decimals: 2,
      });

      expect(answer.of).toMatchObject({ kind: 'answered', exact: '1.1' });
      expect(answer.share).toEqual({ kind: 'divisionByZero' });
      expect(answer.change).toEqual({ kind: 'fromZero' });
      expect(answer.apply).toMatchObject({ exact: '204' });
      expect(answer.before).toMatchObject({ exact: '240' });
    });

    it('answers the usual questions as they are typed', async () => {
      await openTool('percentages');
      await setField(testid('percentages-of-x'), '15');
      await setField(testid('percentages-of-y'), '240');
      await setField(testid('percentages-change-x'), '80');
      await setField(testid('percentages-change-y'), '100');

      await eventually(
        async () => digits(await $(testid('percentages-of-result')).getText()),
        (result) => result === '36',
        '15 % of 240',
      );
      await eventually(
        () => $(testid('percentages-change-result')).getText(),
        (result) => result === '+25 %',
        'from 80 to 100',
      );
      await setField(testid('percentages-share-x'), '36');
      await setField(testid('percentages-share-y'), '0');
      await eventually(
        () => $(testid('percentages-share-problem')).getAttribute('data-problem'),
        (problem) => problem === 'divisionByZero',
        'the division by zero',
      );
      await $(testid('tool-clear')).click();
    });

    it('reads a mode both ways and a umask, in Rust', async () => {
      const answer = await bridge.describePermissions({ mode: 'rwsr-xr-x', umask: '027' });

      expect(answer.mode).toMatchObject({
        kind: 'read',
        mode: { octal: '4755', chmodSymbolic: 'u=rwxs,g=rx,o=rx' },
      });
      expect(answer.umask).toMatchObject({
        kind: 'read',
        file: { octal: '640' },
        directory: { octal: '750' },
      });
    });

    it('keeps the octal, the letters and the boxes in step', async () => {
      await openTool('permissions');
      await setField(testid('permissions-octal'), '644');

      await eventually(
        () => $(testid('permissions-symbolic')).getValue(),
        (letters) => letters === 'rw-r--r--',
        'the letters',
      );
      await $(testid('permissions-owner-execute')).click();
      await eventually(
        () => $(testid('permissions-octal')).getValue(),
        (octal) => octal === '744',
        'the box flipped',
      );
      await setField(testid('permissions-symbolic'), 'drwxrwxrwt');
      await eventually(
        () => $(testid('permissions-octal')).getValue(),
        (octal) => octal === '1777',
        'the sticky directory',
      );
      expect(await $(testid('permissions-type')).getAttribute('data-type')).toBe('directory');
      await setField(testid('permissions-octal'), '758');
      await eventually(
        () => $(testid('permissions-problem')).getAttribute('data-problem'),
        (problem) => problem === 'notOctal',
        'the digit refused',
      );
      await $(testid('tool-clear')).click();
    });

    it('says a negative size is refused, and where', async () => {
      await openTool('sizes');
      await setField(testid('sizes-input'), '-5 MB');

      await eventually(
        () => $(testid('sizes-problem')).getAttribute('data-problem'),
        (problem) => problem === 'negative',
        'the refusal',
      );
      await $(testid('tool-clear')).click();
    });
  });

  describe('comparing JSON', () => {
    const STAGING = JSON.stringify({
      service: 'billing',
      replicas: 2,
      database: { host: 'db.staging.internal', pool: 10 },
      features: { newInvoices: true, betaExports: true },
      logLevel: 'debug',
    });
    const PRODUCTION = JSON.stringify({
      replicas: 6,
      service: 'billing',
      database: { host: 'db.prod.internal', pool: 40, readReplica: 'db-ro.prod.internal' },
      features: { newInvoices: true },
      logLevel: 'info',
      sentry: { sampleRate: 0.2 },
    });
    let noteId = '';

    after(async () => {
      await bridge.deleteNotes([noteId]);
      await bridge.purgeNotes([noteId]);
    });

    it('answers the changes and the patch from Rust', async () => {
      const answer = await bridge.diffJson({
        a: STAGING,
        b: PRODUCTION,
        ignoreKeyOrder: true,
        ignoreWhitespace: true,
      });

      expect(answer.kind).toBe('compared');
      if (answer.kind !== 'compared') return;
      expect(answer.counts).toEqual({ added: 2, removed: 1, modified: 4 });
      expect(JSON.parse(answer.patch)).toContainEqual({ op: 'replace', path: '/replicas', value: 6 });
    });

    it('lists the mockup’s seven changes, and brings one into view', async () => {
      await openTool('json-diff');
      await setField(testid('json-diff-a'), STAGING);
      await setField(testid('json-diff-b'), PRODUCTION);

      await eventually(
        () => readEach(testid('json-diff-change'), '@data-path'),
        (paths) => paths.length === 7,
        'seven changes',
      );
      expect(await readEach(testid('json-diff-change'), '@data-path')).toEqual([
        '$.replicas',
        '$.database.host',
        '$.database.pool',
        '$.database.readReplica',
        '$.features.betaExports',
        '$.logLevel',
        '$.sentry',
      ]);

      await $(`${testid('json-diff-change')}[data-path="$.database.pool"]`).click();
      await $(`${testid('json-diff-row')}.selected`).waitForExist({ timeout: 5_000 });
    });

    it('counts a new key order only when asked', async () => {
      await $(testid('json-diff-ignore-order')).click();

      await eventually(
        () => readEach(testid('json-diff-change'), '@data-kind'),
        (kinds) => kinds.includes('reordered'),
        'the reordering',
      );
      await $(testid('json-diff-ignore-order')).click();
    });

    it('opens a JSON snippet into A, under its title', async () => {
      noteId = (
        await bridge.createNote(
          draft({ spaceId, title: 'config.staging.json', content: STAGING, language: 'json' }),
        )
      ).id;

      await $(testid('json-diff-open-a')).click();
      await setField(testid('json-note-search'), 'config.staging');
      await $(`${testid('json-note-option')}[data-note-id="${noteId}"]`).click();

      await eventually(
        () => readEach(testid('json-diff-name-a'), 'text'),
        (names) => names[0] === 'config.staging.json',
        'the snippet in A',
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
