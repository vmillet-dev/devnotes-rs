import { $, $$, browser, expect } from '@wdio/globals';
import { mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { deflateSync } from 'node:zlib';

import { canvas } from '../pageobjects/canvas.page.js';
import {
  activeTestId,
  clipboardText,
  eventually,
  pickChoice,
  preferenceWritten,
  press,
  readEach,
  reloadCanvas,
  setField,
  testid,
  viewportSize,
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

/** The names as the application says them: read from its own files, so a rename never touches a spec. */
function toolNames(locale: string): Record<string, { name: string }> {
  const file = new URL(`../../src/app/core/services/i18n/translations/${locale}.json`, import.meta.url);
  return (
    JSON.parse(readFileSync(fileURLToPath(file), 'utf8')) as { tools: Record<string, { name: string }> }
  ).tools;
}

/** A real PNG of that size, opaque white: what a WebView draws, and what Rust sizes by its header. */
function pngOf(width: number, height: number): Buffer {
  const crc = (bytes: Buffer): number => {
    let sum = 0xffffffff;
    for (const byte of bytes) {
      sum ^= byte;
      for (let bit = 0; bit < 8; bit += 1) sum = (sum >>> 1) ^ (sum & 1 ? 0xedb88320 : 0);
    }
    return (sum ^ 0xffffffff) >>> 0;
  };
  const chunk = (type: string, data: Buffer): Buffer => {
    const body = Buffer.concat([Buffer.from(type, 'ascii'), data]);
    const [length, sum] = [Buffer.alloc(4), Buffer.alloc(4)];
    length.writeUInt32BE(data.length);
    sum.writeUInt32BE(crc(body));
    return Buffer.concat([length, body, sum]);
  };
  const header = Buffer.alloc(13);
  header.writeUInt32BE(width, 0);
  header.writeUInt32BE(height, 4);
  header.set([8, 6, 0, 0, 0], 8);
  // Each row: its filter byte, none, then four bytes a pixel.
  const rows = Buffer.alloc(height * (1 + width * 4), 0xff);
  for (let row = 0; row < height; row += 1) rows[row * (1 + width * 4)] = 0;
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk('IHDR', header),
    chunk('IDAT', deflateSync(rows)),
    chunk('IEND', Buffer.alloc(0)),
  ]);
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
    // A space of its own, so what it asserts reads only what this file wrote.
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

    it('takes Ctrl+K to its search too, from the page as from an open tool', async () => {
      await $(testid('tools-count')).click();
      await press('k', ['Control']);
      await eventually(activeTestId, (id) => id === 'tools-search', 'the search field to take Ctrl+K');

      await $(`${testid('tools-entry')}[data-tool="case"]`).click();
      await $(testid('tool-back')).waitForExist({ timeout: 10_000 });
      await press('k', ['Control']);
      await eventually(
        activeTestId,
        (id) => id === 'tools-search',
        'the home and its search, from an open tool',
      );
    });

    it('says what it holds reaches no network in its subtitle, with no band above the panels', async () => {
      expect(await $(testid('tools-count')).getAttribute('title')).not.toBe('');
      expect(await $(testid('tools-promise')).isExisting()).toBe(false);
      // The panels come right under the header.
      expect(
        await browser.execute(() => {
          const header = document.querySelector('app-tools-home .topbar')!.getBoundingClientRect();
          const body = document.querySelector('app-tools-home .body')!.getBoundingClientRect();
          return Math.round(body.top - header.bottom);
        }),
      ).toBe(0);
    });

    it('finds a tool by one of its keywords, in any case', async () => {
      await setField(testid('tools-search'), 'CrLf');

      await eventually(
        () => readEach(testid('tools-entry'), '@data-tool'),
        (tools) => tools.join() === 'line-breaks',
        'the search to narrow',
      );
    });

    it('finds the case converter by one of its new cases', async () => {
      await setField(testid('tools-search'), 'train');

      await eventually(
        () => readEach(testid('tools-entry'), '@data-tool'),
        (tools) => tools.includes('case'),
        'Train-Case to find it',
      );
      await setField(testid('tools-search'), '');
    });

    /** A word of the name a tool had before its rename still leads to it, as its new name does. */
    it('finds a renamed tool by a word of its old name and by its new one', async () => {
      const names = toolNames(await browser.execute(() => document.documentElement.lang));
      const renamed = [
        ['checks', 'contrôle'],
        ['base64', 'bases'],
        ['convert', 'toml'],
      ] as const;

      for (const [tool, oldWord] of renamed) {
        for (const wanted of [oldWord, names[tool]!.name]) {
          await setField(testid('tools-search'), wanted);
          await eventually(
            () => readEach(testid('tools-entry'), '@data-tool'),
            (tools) => tools.includes(tool),
            `« ${wanted} » to find ${tool}`,
          );
        }
      }
      await setField(testid('tools-search'), '');
    });

    it('says so when no tool answers a search', async () => {
      await setField(testid('tools-search'), 'zzz-no-such-tool');

      await $(testid('tools-no-match')).waitForDisplayed({ timeout: 5_000 });
      await setField(testid('tools-search'), '');
    });

    /**
     * « generateur » found nothing, though three tools sit under « Générateurs »; « 418 » found
     * nothing, though the status reference holds it; Enter did nothing; and the rail of an open
     * tool listed four recents where the home listed five.
     */
    it('finds a category by its name and an entry inside a reference, opens it on Enter, and keeps five recents in the rail', async () => {
      const words = toolNames(await browser.execute(() => document.documentElement.lang)) as unknown as {
        categories: Record<string, string>;
      };
      const folded = words.categories['generate']!.normalize('NFD').replace(/\p{M}/gu, '').toLowerCase();
      const generators = await readEach(
        `${testid('tools-panel')}[data-category="generate"] ${testid('tools-entry')}`,
        '@data-tool',
      );
      expect(generators.length).toBeGreaterThan(2);

      // The folded name of a category finds every tool filed under it.
      await setField(testid('tools-search'), folded.slice(0, -1));
      await eventually(
        () => readEach(testid('tools-entry'), '@data-tool'),
        (tools) => generators.every((tool) => tools.includes(tool)) && tools.length < 2 * generators.length,
        'the generators found by the name of their category',
      );

      // « 418 » is no tool: it is a row of the status reference, listed under the tools.
      const hit = `${testid('tools-reference-hit')}[data-tool="http-status"][data-key="418"]`;
      await setField(testid('tools-search'), '418');
      await $(hit).waitForDisplayed({ timeout: 10_000 });
      expect(await $(hit).getText()).toContain('418');
      expect(await readEach(testid('tools-entry'), '@data-tool')).toEqual([]);
      expect(await $(`${hit}[data-first]`).isExisting()).toBe(true);
      expect(await $(testid('tools-no-match')).isExisting()).toBe(false);

      // The arrows go down into the results and back up; Enter opens the first from the field.
      const focused = () =>
        browser.execute(() => {
          const active = document.activeElement as HTMLElement | null;
          return active?.dataset['key'] ?? active?.dataset['testid'] ?? '';
        });
      await $(testid('tools-search')).click();
      await press('ArrowDown');
      await eventually(focused, (on) => on === '418', 'the first result focused');
      await press('ArrowUp');
      await eventually(focused, (on) => on === 'tools-search', 'the field focused again');
      await press('Enter');

      // The reference opens on that entry: its table whole, the row lit and in view.
      await eventually(
        () => $(testid('tool-body')).getAttribute('data-tool'),
        (shown) => shown === 'http-status',
        'the status reference to open',
      );
      const row = `${testid('reference-row')}[data-key="418"]`;
      await $(`${row}[data-lit]`).waitForExist({ timeout: 10_000 });
      expect(await $(testid('reference-search')).getValue()).toBe('');
      await eventually(
        () =>
          browser.execute((lit: string) => {
            const box = document.querySelector(lit)!.getBoundingClientRect();
            return box.top >= 0 && box.bottom <= window.innerHeight;
          }, row),
        (inView) => inView,
        'the row brought into view',
      );
      // Its table scrolled, not the window: the tool's header is where it was.
      expect(
        await browser.execute(
          (back: string) => document.querySelector(back)!.getBoundingClientRect().top,
          testid('tool-back'),
        ),
      ).toBeGreaterThanOrEqual(0);
      await $(testid('tool-back')).click();
      await setField(testid('tools-search'), '');

      // Six tools of other categories, then a reference: its rail lists nine tools above, and
      // still the five opened last under them — the home its own five, the open tool first.
      const opened = ['case', 'hash', 'url-codec', 'lorem', 'dates', 'cidr'];
      for (const tool of opened) await openTool(tool);
      await openTool('http-status');
      await eventually(
        () => readEach(testid('tools-rail-recent'), '@data-tool'),
        (recents) => recents.join() === [...opened].reverse().slice(0, 5).join(),
        'five recents in the rail of a reference',
      );
      await $(testid('tool-back')).click();
      await eventually(
        () => readEach(testid('tools-recent'), '@data-tool'),
        (recents) => recents.join() === ['http-status', ...[...opened].reverse().slice(0, 4)].join(),
        'five recents on the home',
      );
    });

    /**
     * Thirty-two tools, and the five used every day were found again each time through a
     * category or the search: « Récents » moves with every tool opened.
     */
    it('stars a tool from its header and another from its row, finds them after a refresh, and narrows the home to them', async () => {
      const favourites = () => readEach(testid('tools-favourite'), '@data-tool');
      const entries = () => readEach(testid('tools-entry'), '@data-tool');
      const star = (tool: string) => $(`${testid('tools-entry-star')}[data-tool="${tool}"]`);
      const starred = () => readEach(`${testid('tools-entry-star')}[aria-pressed="true"]`, '@data-tool');

      // Whatever a run before left, nothing is starred to begin with.
      for (const tool of await starred()) await star(tool).click();
      await eventually(starred, (none) => none.length === 0, 'nothing starred to begin with');
      expect(await $(testid('tools-rail-favourites')).isExisting()).toBe(false);
      expect(await favourites()).toEqual([]);

      // From the tool's own header, then from a row of the home without opening that tool.
      await openTool('hash');
      await $(testid('tool-favourite')).click();
      await eventually(
        () => $(testid('tool-favourite')).getAttribute('aria-pressed'),
        (pressed) => pressed === 'true',
        'the star of the open tool on',
      );
      await $(testid('tool-back')).click();
      await star('url-codec').click();
      await eventually(
        favourites,
        (tools) => tools.join() === 'hash,url-codec',
        'the two favourites, in the order they were starred',
      );
      expect(await $(testid('tool-body')).isExisting()).toBe(false);
      // A favourite has its chip above: it is not repeated among the recents.
      expect(await readEach(testid('tools-recent'), '@data-tool')).not.toContain('hash');

      // Kept with the application: the page built again, they are where they were.
      await preferenceWritten('devnotes.favouriteTools', JSON.stringify(['hash', 'url-codec']));
      await reloadCanvas();
      await $(testid('tools-page')).waitForDisplayed({ timeout: 10_000 });
      await eventually(
        favourites,
        (tools) => tools.join() === 'hash,url-codec',
        'the favourites after a refresh',
      );

      // The rail counts them, and narrows the home to them as a category does.
      expect(await $(testid('tools-rail-favourites')).getText()).toContain('2');
      await $(testid('tools-rail-favourites')).click();
      await eventually(entries, (tools) => tools.join() === 'hash,url-codec', 'the home narrowed to them');

      // Unstarred from their rows: the entry goes with the last one, and the catalogue is whole again.
      await star('hash').click();
      await eventually(entries, (tools) => tools.join() === 'url-codec', 'one favourite left');
      await star('url-codec').click();
      await $(testid('tools-rail-favourites')).waitForExist({ reverse: true, timeout: 10_000 });
      await eventually(entries, (tools) => tools.length > 20, 'the whole catalogue again');
      expect(await favourites()).toEqual([]);
    });

    /** The slug is a row of the case converter, and the URL parser a tab of the URL tool; the diff joined from Comparer. */
    it('lays the Texte panel out with its four tools', async () => {
      expect(
        await readEach(
          `${testid('tools-panel')}[data-category="text"] ${testid('tools-entry')}`,
          '@data-tool',
        ),
      ).toEqual(['case', 'line-breaks', 'text-stats', 'diff']);
    });
  });

  /**
   * The panels pack by column. A grid left a hole under every panel shorter than its row's
   * tallest; jsdom measures no layout, so only the assembled window can say.
   */
  describe('their home, at any width', () => {
    const GAP = 14;
    let restore = { width: 1100, height: 720 };

    interface PanelBox {
      readonly left: number;
      readonly top: number;
      readonly bottom: number;
      readonly width: number;
      /** More than one is a panel cut across two columns. */
      readonly pieces: number;
    }

    const boxes = (): Promise<PanelBox[]> =>
      browser.execute(
        (selector: string) =>
          [...document.querySelectorAll(selector)].map((panel) => {
            const box = panel.getBoundingClientRect();
            return {
              left: Math.round(box.left),
              top: box.top,
              bottom: box.bottom,
              width: box.width,
              pieces: panel.getClientRects().length,
            };
          }),
        testid('tools-panel'),
      );

    const columnsOf = (panels: readonly PanelBox[]): PanelBox[][] => {
      const columns = new Map<number, PanelBox[]>();
      for (const panel of panels) {
        columns.set(panel.left, [...(columns.get(panel.left) ?? []), panel]);
      }
      return [...columns.values()].map((column) => column.sort((a, b) => a.top - b.top));
    };

    /** Where a panel does not start right under the one above it. */
    const holes = (panels: readonly PanelBox[]): number[] =>
      columnsOf(panels).flatMap((column) =>
        column
          .slice(1)
          .map((panel, index) => panel.top - column[index]!.bottom)
          .filter((space) => Math.abs(space - GAP) > 1),
      );

    /** The home's columns follow a measured width a render later: as many as fit at 260 px. */
    const columnsFollowTheWidth = (): Promise<boolean> =>
      browser.execute(
        (selector: string, gap: number) => {
          const area = document.querySelector<HTMLElement>(selector);
          // An open tool hides the home: no columns to wait for.
          if (!area) return true;
          const declared = Number(area.style.getPropertyValue('--columns'));
          return declared === Math.max(1, Math.floor((area.clientWidth + gap) / (260 + gap)));
        },
        testid('tools-panels'),
        GAP,
      );

    async function resizeTo(width: number): Promise<PanelBox[]> {
      await browser.setWindowSize(width, 800);
      await eventually(
        viewportSize,
        (size) => size.width <= width && size.width > width - 60,
        `the window to be ${width}px wide`,
      );
      await eventually(columnsFollowTheWidth, (follows) => follows, `the columns to follow ${width}px`);
      return boxes();
    }

    before(async () => {
      restore = await browser.getWindowSize();
      await $(testid('tools-rail-all')).click();
      await setField(testid('tools-search'), '');
    });

    after(async () => {
      await $(testid('tools-rail-all')).click();
      await browser.setWindowSize(restore.width, restore.height);
    });

    it('leaves no hole between two panels of a column, and cuts none in two', async () => {
      const columnCounts: number[] = [];

      for (const width of [700, 1000, 1280]) {
        const panels = await resizeTo(width);

        expect(panels.length).toBeGreaterThan(5);
        expect(holes(panels)).toEqual([]);
        expect(panels.filter((panel) => panel.pieces !== 1)).toEqual([]);
        columnCounts.push(columnsOf(panels).length);
      }

      expect(columnCounts[0]).toBe(1);
      expect(columnCounts[2]).toBeGreaterThanOrEqual(3);
    });

    /** The count gives way first: cut on its line, not wrapped under the title. */
    it('keeps its top bar on one line at the narrowest', async () => {
      await resizeTo(700);

      const lines = await browser.execute((count: string) => {
        const text = document.querySelector<HTMLElement>(count)!;
        return Math.round(text.getBoundingClientRect().height / parseFloat(getComputedStyle(text).fontSize));
      }, testid('tools-count'));

      expect(lines).toBe(1);
    });

    it('packs what a search leaves the same way', async () => {
      await resizeTo(1000);
      await setField(testid('tools-search'), 'json');
      const panels = await eventually(
        boxes,
        (found) => found.length >= 2 && found.length < 7,
        'the search to leave a few panels',
      );

      expect(holes(panels)).toEqual([]);
      expect(panels.filter((panel) => panel.pieces !== 1)).toEqual([]);
      await setField(testid('tools-search'), '');
    });

    /** Asked on a 2000 px screen: CSS columns filled the left half and left the right one empty. */
    it('spreads its panels over a large screen, and overflows nothing on a laptop', async () => {
      const span = () =>
        browser.execute(
          (home: string, panel: string) => {
            const area = document.querySelector(home)!.getBoundingClientRect();
            const boxes = [...document.querySelectorAll(panel)].map((each) => each.getBoundingClientRect());
            const right = Math.max(...boxes.map((box) => box.right));
            return {
              share: (right - Math.min(...boxes.map((box) => box.left))) / area.width,
              overflow: right - area.right,
              scrolls: document.documentElement.scrollWidth > document.documentElement.clientWidth,
            };
          },
          testid('tools-panels'),
          testid('tools-panel'),
        );

      await resizeTo(1900);
      const wide = await eventually(
        span,
        (measured) => measured.share >= 0.8,
        'the panels to fill the width',
      );
      expect(wide.overflow).toBeLessThanOrEqual(1);

      await resizeTo(1100);
      const laptop = await span();
      expect(laptop.overflow).toBeLessThanOrEqual(1);
      expect(laptop.scrolls).toBe(false);
    });

    it('puts an open tool’s results two abreast when its card has the room', async () => {
      const lefts = () =>
        browser.execute(
          (row: string) =>
            new Set(
              [...document.querySelectorAll(row)].map((each) =>
                Math.round(each.getBoundingClientRect().left),
              ),
            ).size,
          `${testid('case-results')} app-result-row`,
        );

      await resizeTo(2000);
      await entry('case').click();
      await setField(testid('case-input'), 'Hello world');
      expect(await eventually(lefts, (count) => count === 2, 'the results two abreast')).toBe(2);

      await resizeTo(1100);
      expect(await eventually(lefts, (count) => count === 1, 'the results one under another')).toBe(1);
      await $(testid('tools-rail-all')).click();
    });

    it('keeps one category to a column, not stretched over the page', async () => {
      const whole = await resizeTo(1280);
      const columnWidth = whole[0]!.width;

      await $(`${testid('tools-rail-category')}[data-category="calc"]`).click();
      const panels = await eventually(boxes, (found) => found.length === 1, 'one category to show');

      expect(panels[0]!.pieces).toBe(1);
      // The whole home scrolls and one category does not: the scrollbar's width, shared out
      // between the columns, is all that may separate the two.
      expect(Math.abs(panels[0]!.width - columnWidth)).toBeLessThanOrEqual(8);
    });
  });

  describe('the text tools', () => {
    it('converts a text into every case', async () => {
      await openTool('case');

      await setField(testid('case-input'), 'parse HTTP response');

      await eventually(outputValues, (values) => values.length === 14, 'the thirteen cases and the slug');
      // For the code, for the text, then for a URL: three groups in one card.
      expect(await outputValues()).toEqual([
        'parseHttpResponse',
        'ParseHttpResponse',
        'parse_http_response',
        'PARSE_HTTP_RESPONSE',
        'parse-http-response',
        'Parse-Http-Response',
        'parse.http.response',
        'parse/http/response',
        'parsehttpresponse',
        'Parse Http Response',
        'Parse http response',
        'PARSE HTTP RESPONSE',
        'parse http response',
        'parse-http-response',
      ]);
      expect(await readEach(testid('case-group'), '@data-group')).toEqual(['code', 'text', 'url']);
      expect(await readEach(`${testid('case-words')} li`, 'text')).toEqual(['parse', 'HTTP', 'response']);
    });

    /** Rust's options, through the bridge: accents leave the code cases alone, French titles. */
    it('strips accents from the code cases only, and writes a French title', async () => {
      await setField(testid('case-input'), "le guide de l'été");
      const value = (name: string) =>
        $(`${testid('output-row')}[data-name="${name}"] ${testid('output-value')}`).getText();

      await eventually(
        () => value('snake_case'),
        (snake) => snake === 'le_guide_de_l_ete',
        'accents stripped',
      );
      expect(await value('Title Case')).toBe("Le Guide De L'été");

      await $(`${testid('segmented-case-title-language')} [data-segment-id="french"]`).click();
      await eventually(
        () => value('Title Case'),
        (title) => title === "Le Guide de l'Été",
        'a French title',
      );

      await $(testid('case-strip-accents')).click();
      await eventually(
        () => value('snake_case'),
        (snake) => snake === 'le_guide_de_l_été',
        'accents kept',
      );

      await $(testid('case-strip-accents')).click();
      await $(`${testid('segmented-case-title-language')} [data-segment-id="english"]`).click();
    });

    it('converts a list line by line', async () => {
      await setField(testid('case-input'), 'userId\nHTTPServer\nutf8Decoder');

      await eventually(
        () => $(`${testid('output-row')}[data-name="snake_case"] ${testid('output-value')}`).getText(),
        (snake) => snake === 'user_id\nhttp_server\nutf8_decoder',
        'three lines in snake_case',
      );
      await setField(testid('case-input'), 'parse HTTP response');
      await eventually(outputValues, (values) => values[0] === 'parseHttpResponse', 'one line again');
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

    it('transliterates the slug where the code cases only strip accents', async () => {
      await setField(testid('case-input'), 'Été 2026 ! Straße');
      const value = (name: string) =>
        $(`${testid('output-row')}[data-name="${name}"] ${testid('output-value')}`).getText();

      await eventually(
        () => value('Slug'),
        (slug) => slug === 'ete-2026-strasse',
        'the slug',
      );
      expect(await value('snake_case')).toBe('ete_2026_straße');

      await $(`${testid('segmented-case-slug-separator')} [data-segment-id="underscore"]`).click();
      await eventually(
        () => value('Slug'),
        (slug) => slug === 'ete_2026_strasse',
        'the slug with underscores',
      );
      await $(`${testid('segmented-case-slug-separator')} [data-segment-id="dash"]`).click();
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

    it('counts characters as a reader sees them, in Rust', async () => {
      expect(
        await bridge.textStats({ text: 'e\u0301te\u0301 🇫🇷', foldCase: false, countWhitespace: false }),
      ).toMatchObject({ characters: 5, codePoints: 8, utf16Units: 10, utf8Bytes: 16, words: 1 });
    });

    it('counts a text and names the invisible character in it', async () => {
      await $(`${testid('tools-rail-tool')}[data-tool="text-stats"]`).click();
      await setField(testid('text-stats-input'), 'Deux mots\u00a0!');

      await eventually(
        () => $(`[data-count="words"] ${testid('text-stats-value')}`).getText(),
        (words) => words === '2',
        'two words',
      );
      await $(testid('text-stats-whitespace')).click();
      await eventually(
        () => readEach(testid('text-stats-frequency'), '@data-invisible'),
        (invisible) => invisible.includes('noBreakSpace'),
        'the no-break space named',
      );
      await $(testid('text-stats-whitespace')).click();
    });

    it('lists the tools opened last on the home, most recent first', async () => {
      await $(testid('tool-back')).click();

      await eventually(
        () => readEach(testid('tools-recent'), '@data-tool'),
        (tools) => tools.slice(0, 3).join() === 'text-stats,line-breaks,case',
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

    it('offers a decoded URL to its Analyser tab, which takes it apart', async () => {
      await setField(
        testid('url-codec-encoded'),
        'https%3A%2F%2Fapi.exemple.fr%3A8443%2Fv1%3Fstatut%3Dpay%25C3%25A9e%23bas',
      );
      await $(testid('url-analyse-this')).waitForDisplayed({ timeout: 5_000 });
      await $(testid('url-analyse-this')).click();

      await $(testid('url-parser-parameters')).waitForDisplayed({ timeout: 5_000 });
      expect(await $(testid('url-parser-input')).getValue()).toBe(
        'https://api.exemple.fr:8443/v1?statut=pay%C3%A9e#bas',
      );
      expect(await $(testid('url-parser-parameters')).getText()).toContain('payée');
      expect(await $('[data-part="port"]').getText()).toContain('8443');
    });

    it('says why a URL is refused, and keeps the text on the way back', async () => {
      await setField(testid('url-parser-input'), 'exemple.fr/chemin');
      await $(testid('url-parser-problem')).waitForDisplayed({ timeout: 5_000 });

      await $(`${testid('segmented-url-tab')} [data-segment-id="codec"]`).click();
      expect(await $(testid('url-codec-decoded')).getValue()).toBe('exemple.fr/chemin');
      await $(testid('tool-clear')).click();
    });

    const encoding = (id: string) => $(`[data-encoding="${id}"] ${testid('output-value')}`).getText();

    it('writes a text in every encoding at once, Base64 padded and Base64 URL not', async () => {
      await openTool('base64');
      await setField(testid('base64-input'), 'été?>');
      await eventually(
        () => encoding('base64'),
        (text) => text === 'w6l0w6k/Pg==',
        'standard',
      );

      expect(await encoding('base64Url')).toBe('w6l0w6k_Pg');
      expect(await encoding('hex')).toBe('c3 a9 74 c3 a9 3f 3e');
    });

    it('decodes what it recognises, and names the character a decoding stops at', async () => {
      await $(`${testid('segmented-base64-direction')} [data-segment-id="decode"]`).click();
      await setField(testid('base64-input'), '43 61 66 c3 a9');
      await eventually(
        () => $(testid('base64-reading')).getAttribute('data-reading'),
        (reading) => reading === 'hex',
        'read as hexadecimal',
      );
      expect(await $(testid('base64-output')).getText()).toBe('Café');

      await setField(testid('base64-input'), 'QUJD#');
      await $(testid('base64-problem')).waitForDisplayed({ timeout: 5_000 });
      expect(await $(testid('base64-problem')).getText()).toContain('#');
      await $(testid('tool-clear')).click();
      await $(`${testid('segmented-base64-direction')} [data-segment-id="encode"]`).click();
    });

    /**
     * Decoded bytes that are a picture could only be saved, then opened elsewhere to see what
     * they were; and a long result ran down a strip in the middle of an empty window.
     */
    it('shows the picture a decoding is, a data: URI included, and gives a long result the width of the window', async () => {
      const picture = pngOf(24, 16).toString('base64');
      const drawn = () =>
        browser.execute((image: string) => {
          const shown = document.querySelector(image) as HTMLImageElement | null;
          return shown?.complete ? `${shown.naturalWidth} × ${shown.naturalHeight}` : '';
        }, testid('base64-image-picture'));
      await $(`${testid('segmented-base64-direction')} [data-segment-id="decode"]`).click();

      // Rust names and sizes it by its header; the WebView draws it, under the CSP as it stands.
      await setField(testid('base64-input'), picture);
      await $(testid('base64-image')).waitForDisplayed({ timeout: 10_000 });
      expect(await $(testid('base64-image')).getAttribute('data-format')).toBe('png');
      expect(await $(testid('base64-image-size')).getText()).toBe('24 × 16 px');
      await eventually(drawn, (size) => size === '24 × 16', 'the picture drawn at its own size');

      // Pasted whole with its header, it is read by it rather than refused.
      await setField(testid('base64-input'), `data:image/png;base64,${picture}`);
      await $(testid('base64-data-uri')).waitForDisplayed({ timeout: 10_000 });
      expect(await $(testid('base64-problem')).isExisting()).toBe(false);
      expect(await $(testid('base64-image-size')).getText()).toBe('24 × 16 px');

      // Text that is no picture shows none.
      await setField(testid('base64-input'), 'Q2Fmw6k=');
      await $(testid('base64-image')).waitForExist({ reverse: true, timeout: 10_000 });
      await $(`${testid('segmented-base64-direction')} [data-segment-id="encode"]`).click();

      // A long text on a wide window: the first result takes more than half the tool, held to
      // a height, and the second encoding is still in view under it.
      const before = await browser.getWindowSize();
      await browser.setWindowSize(1400, 800);
      await eventually(viewportSize, (size) => size.width > 1300, 'a wide window');
      await setField(testid('base64-input'), 'DevNotes garde tout sur cette machine. '.repeat(120));
      await eventually(
        () => encoding('base64'),
        (text) => text.length > 3000,
        'a long Base64',
      );
      const boxes = await browser.execute(
        (first: string, second: string, tool: string) => {
          const box = (selector: string) => document.querySelector(selector)!.getBoundingClientRect();
          const [long, next, whole] = [box(first), box(second), box(tool)];
          return {
            share: long.width / whole.width,
            height: long.height,
            nextTop: next.top,
            nextBottom: next.bottom,
            viewport: window.innerHeight,
          };
        },
        `[data-encoding="base64"] ${testid('output-value')}`,
        `[data-encoding="base64Url"] ${testid('output-row')}`,
        'app-base64-tool',
      );
      expect(boxes.share).toBeGreaterThan(0.5);
      expect(boxes.height).toBeLessThan(220);
      expect(boxes.nextTop).toBeGreaterThan(0);
      expect(boxes.nextBottom).toBeLessThanOrEqual(boxes.viewport);
      await browser.setWindowSize(before.width, before.height);
      await $(testid('tool-clear')).click();
    });

    it('reads a file where it lies, by its path', async () => {
      const folder = mkdtempSync(join(tmpdir(), 'devnotes-e2e-'));
      const path = join(folder, 'logo.bin');
      writeFileSync(path, Buffer.from([0, 255, 16]));

      expect(await bridge.encodeBase64File(path)).toEqual({
        kind: 'encoded',
        name: 'logo.bin',
        bytes: 3,
        base64: 'AP8Q',
        base64Url: 'AP8Q',
      });
      expect(await bridge.encodeBase64File(join(folder, 'absent.bin'))).toEqual({
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
        async () => (await $(testid('colour-ratio')).getText()).replace(',', '.'),
        (ratio) => ratio === '4.48:1',
        'the ratio',
      );
      expect(await $(testid('colour-verdict')).getAttribute('data-verdict')).toBe('failsNormal');
      expect(await readEach(`${testid('colour-verdicts')} td`, '@class')).toEqual(['', 'pass', '', '']);

      await $(testid('colour-fix')).click();
      await eventually(
        async () => Number((await $(testid('colour-ratio')).getText()).replace(',', '.').replace(':1', '')),
        (ratio) => ratio >= 4.5 && ratio < 4.7,
        'the fix reaching 4.5:1',
      );
      expect(await $(testid('colour-input')).getValue()).toMatch(/^#[0-9a-f]{6}$/);
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

      await $(testid('convert-swap')).click();
      await eventually(
        () => $(testid('convert-output')).getText(),
        (json) => json.startsWith('{') && json.indexOf('service') < json.indexOf('replicas'),
        'the JSON back',
      );
    });

    it('leaves out a key TOML has no null for, and says where a list holds one', async () => {
      // Back from the swap, JSON is on the right: it goes left once TOML has taken its place.
      await $(`${testid('segmented-convert-to')} [data-segment-id="toml"]`).click();
      await $(`${testid('segmented-convert-from')} [data-segment-id="json"]`).click();
      await setField(testid('convert-input'), '{"database":{"replica":null,"pool":10}}');

      await eventually(
        () => $(testid('convert-dropped')).getText(),
        (text) => text.includes('$.database.replica'),
        'the key left out',
      );
      expect(await $(testid('convert-output')).getText()).toContain('pool = 10');

      await setField(testid('convert-input'), '{"replicas":[1,null]}');
      await eventually(
        () => $(testid('convert-impossible')).getText(),
        (text) => text.includes('$.replicas[1]'),
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

    it('fills its input from a YAML snippet, read as YAML, and lists no snippet it cannot read', async () => {
      const deploy = await bridge.createNote(
        draft({ spaceId, title: 'deploy.yml', content: 'service: billing\nreplicas: 2\n', language: 'yml' }),
      );
      const query = await bridge.createNote(
        draft({ spaceId, title: 'replicas.sql', content: 'select 1;', language: 'sql' }),
      );

      await $(testid('note-import-text')).click();
      await $(`${testid('note-picker-option')}[data-note-id="${deploy.id}"]`).waitForExist({
        timeout: 10_000,
      });
      expect(await readEach(testid('note-picker-option'), '@data-note-id')).not.toContain(query.id);
      await setField(testid('note-picker-search'), 'deploy');
      await $(`${testid('note-picker-option')}[data-note-id="${deploy.id}"]`).click();

      await eventually(
        () => $(`${testid('segmented-convert-from')} [aria-checked="true"]`).getAttribute('data-segment-id'),
        (format) => format === 'yaml',
        'the format of the snippet',
      );
      await eventually(
        () => $(testid('convert-output')).getText(),
        (json) => JSON.stringify(JSON.parse(json)) === JSON.stringify({ service: 'billing', replicas: 2 }),
        'the snippet converted',
      );
      expect(await $(testid('note-picker')).isExisting()).toBe(false);
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

    it('reads a signature pasted over no text by its shape', async () => {
      await setField(testid('hash-input'), '');

      await eventually(
        async () => (await $(testid('hash-shape')).getText()).trim(),
        (shape) => shape.startsWith('64') && shape.includes('SHA-256') && shape.includes('SHA3-256'),
        'the shape of a 256-bit digest',
      );
      expect(await $(testid('hash-verdict')).isExisting()).toBe(false);

      await setField(testid('hash-input'), 'The quick brown fox jumps over the lazy dog');
      await eventually(
        () => $(testid('hash-verdict')).getText(),
        (verdict) => verdict.includes('HMAC-SHA256'),
        'the match again',
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

    const drawnLines = () => readEach(testid('identifiers-value'), 'text');
    const pickKind = (kind: string) =>
      $(`${testid('segmented-identifiers-kind')} [data-segment-id="${kind}"]`).click();

    it('draws v7 UUIDs that sort by creation, and reads the time back out of one', async () => {
      await openTool('identifiers');
      await pickKind('uuidV7');

      await eventually(
        drawnLines,
        (lines) => lines.length === 5 && lines.every((line) => /^[0-9a-f]{8}-[0-9a-f]{4}-7/.test(line)),
        'five v7s',
      );
      const lines = await drawnLines();
      expect([...lines].sort()).toEqual(lines);

      await setField(testid('identifiers-checked'), '01922b6e-4b30-7cc4-9a5c-6f2d8e1b3a77');
      await eventually(
        () => $(testid('identifiers-created')).getText(),
        (created) => created === '2024-09-25 23:05:01.488 UTC',
        'its time',
      );
      expect((await $(testid('identifiers-random')).getText()).trim()).toBe(
        '7cc4-9a5c-6f2d8e1b3a77 · 74 bits',
      );
    });

    it('draws UUIDs without hyphens, and again on R once out of the field', async () => {
      await $(testid('identifiers-without-hyphens')).click();
      await eventually(
        drawnLines,
        (lines) => lines.length === 5 && lines.every((line) => /^[0-9a-f]{12}7[0-9a-f]{19}$/.test(line)),
        'five v7s of 32 digits',
      );
      const before = await drawnLines();

      await browser.execute(() => (document.activeElement as HTMLElement | null)?.blur());
      await browser.keys('r');
      await eventually(drawnLines, (lines) => lines.length === 5 && lines[0] !== before[0], 'a new draw');
      await $(testid('identifiers-without-hyphens')).click();
    });

    it('draws ULIDs that count up, and reads the ULID spec’s example back', async () => {
      await pickKind('ulid');
      await setField(testid('identifiers-count'), '50');

      await eventually(
        drawnLines,
        (lines) => lines.length === 50 && lines.every((line) => /^[0-9a-z]{26}$/.test(line)),
        'fifty ULIDs',
      );
      const lines = await drawnLines();
      expect([...lines].sort()).toEqual(lines);

      await setField(testid('identifiers-checked'), '01ARZ3NDEKTSV4RRFFQ69G5FAV');
      await eventually(
        () => $(testid('identifiers-created')).getText(),
        (created) => created === '2016-07-30 23:54:10.259 UTC',
        'the ULID’s millisecond',
      );
    });

    it('draws NanoIDs in the alphabet and length chosen, and reads a KSUID', async () => {
      await setField(testid('identifiers-count'), '5');
      await pickKind('nanoId');
      await setField(testid('identifiers-nano-length'), '12');
      await $(`${testid('segmented-identifiers-alphabet')} [data-segment-id="digits"]`).click();

      await eventually(
        drawnLines,
        (lines) => lines.length === 5 && lines.every((line) => /^\d{12}$/.test(line)),
        'twelve digits each',
      );
      await setField(testid('identifiers-checked'), '0ujtsYcgvSTl8PAuAdqWYSMnLOv');
      await eventually(
        () => $(testid('identifiers-facts')).getAttribute('data-kind'),
        (kind) => kind === 'ksuid',
        'the KSUID',
      );
      await pickKind('uuidV4');
      await $(testid('tool-clear')).click();
    });
  });

  describe('JWT', () => {
    const JWT_IO =
      'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c';
    const verification = () => $(testid('jwt-verification')).getAttribute('data-verification');

    it('verifies jwt.io’s example with its secret, in Rust', async () => {
      expect(
        await bridge.decodeJwt({ token: JWT_IO, secret: 'your-256-bit-secret', secretIsBase64: false }),
      ).toMatchObject({ kind: 'decoded', algorithm: 'HS256', verification: 'valid', state: 'undated' });
    });

    it('decodes a token and verifies its signature with a masked secret', async () => {
      await openTool('jwt');
      await setField(testid('jwt-token'), JWT_IO);
      await eventually(verification, (state) => state === 'notAsked', 'nothing verified yet');
      expect(await $(testid('jwt-payload')).getText()).toContain('John Doe');

      await setField(testid('jwt-secret'), 'your-256-bit-secret');
      expect(await $(testid('jwt-secret')).getAttribute('type')).toBe('password');
      await eventually(verification, (state) => state === 'valid', 'a valid signature');

      await setField(testid('jwt-secret'), 'another secret');
      await eventually(verification, (state) => state === 'invalid', 'an invalid signature');
    });

    it('warns before a token is kept as a note', async () => {
      await $(testid('tool-save-as-note')).click();

      await $(testid('save-as-note-warning')).waitForDisplayed({ timeout: 5_000 });
      await $(testid('save-as-note-cancel')).click();
      await $(testid('save-as-note')).waitForExist({ reverse: true, timeout: 5_000 });
    });

    it('forgets the token and the secret once the tool is left', async () => {
      await $(testid('tools-rail-all')).click();
      await entry('jwt').click();

      await $(testid('jwt-token')).waitForDisplayed({ timeout: 5_000 });
      expect(await $(testid('jwt-token')).getValue()).toBe('');
      expect(await $(testid('jwt-secret')).getValue()).toBe('');
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

    /**
     * The sample document, with « Schéma » chosen above it, answered three strings of lorem
     * ipsum and a warning nobody understood; and « La garder » stood far from the seed it kept.
     */
    it('says an example is no schema and reads it as a model, loads the sample of the mode, and pins the seed it drew', async () => {
      const source = (id: string) =>
        $(`${testid('segmented-json-generator-source')} [data-segment-id="${id}"]`);
      const documents = async () => JSON.parse(await output()) as unknown[];
      await $(testid('tool-clear')).click();

      // The sample of « Modèle » is a document, and documents like it come out.
      await source('example').click();
      await $(testid('tool-sample')).click();
      await eventually(output, (text) => text.includes('"createdAt"'), 'documents like the sample');

      // « Schéma » does not take it for a schema: nothing is drawn, and the way back is offered.
      await source('schema').click();
      await $(testid('json-generator-an-example')).waitForDisplayed({ timeout: 10_000 });
      expect(await $(testid('json-generator-output')).isExisting()).toBe(false);
      expect(await $(testid('json-generator-unsupported')).isExisting()).toBe(false);
      await $(testid('json-generator-as-example')).click();
      await eventually(output, (text) => text.includes('"createdAt"'), 'the documents, read as a model');
      expect(await $(testid('json-generator-expects')).getAttribute('data-source')).toBe('example');

      // « Exemple » under « Schéma » fills a schema: documents come out of it, with no warning.
      await source('schema').click();
      await $(testid('json-generator-an-example')).waitForDisplayed({ timeout: 10_000 });
      await $(testid('tool-sample')).click();
      await eventually(
        () => $(testid('json-generator-input')).getValue(),
        (text) => text.includes('"properties"'),
        'the sample of the schema mode',
      );
      // A required property: the others are drawn one time in two, and three documents may have none.
      await eventually(output, (text) => text.includes('"email"'), 'documents drawn from the schema');
      expect(await $(testid('json-generator-unsupported')).isExisting()).toBe(false);

      // « Régénérer » draws others while no seed is pinned…
      const first = await output();
      await $(testid('tool-action-draw')).click();
      const second = await eventually(output, (text) => text !== first, 'other documents');

      // …and the seed just used is said at its own field, with the pin that fixes it there.
      await $(testid('json-generator-seed-used')).waitForDisplayed({ timeout: 5_000 });
      await $(testid('json-generator-keep-seed')).click();
      await $(testid('json-generator-seed-pinned')).waitForDisplayed({ timeout: 5_000 });
      expect(await $(testid('json-generator-seed')).getValue()).toMatch(/^\d+$/);

      // Pinned, « Régénérer » draws the same: two documents asked next are the first two again.
      await $(testid('tool-action-draw')).click();
      await setField(testid('json-generator-count'), '2');
      await eventually(documents, (drawn) => drawn.length === 2, 'two documents');
      expect(await documents()).toEqual((JSON.parse(second) as unknown[]).slice(0, 2));

      await setField(testid('json-generator-count'), '3');
      await $(testid('tool-clear')).click();
    });

    /**
     * A form or a test database was filled with « Jean Dupont » and an IBAN found on a forum: a
     * person whose every number passes its own check, the same again from the same seed.
     */
    it('draws a fictitious person as a card, the same from the same seed, whose IBAN the checks tool finds valid', async () => {
      interface Drawn {
        sex: string;
        surname: string;
        birth: { date: string };
        bank: { iban: string };
        identifiers: { kind: string; value: string }[];
      }
      const iban = () => $(`[data-field="iban"] ${testid('output-value')}`).getText();
      // ⚠️ The clipboard may still hold the JSON of a scenario before: this person's is the one with their IBAN.
      const person = async (shown: string): Promise<Drawn> => {
        await $(testid('identity-copy-json')).click();
        const json = await eventually(
          clipboardText,
          (text) => text?.includes(`"${shown}"`) === true,
          'the person as JSON',
        );
        return JSON.parse(json ?? '{}') as Drawn;
      };
      await openTool('identity');
      await $(testid('identity-card')).waitForDisplayed({ timeout: 10_000 });

      // The sample's seed draws the same person for everyone.
      await $(testid('tool-sample')).click();
      await eventually(
        () => $(testid('identity-seed')).getValue(),
        (seed) => seed === 'devnotes',
        'the seed of the sample',
      );
      await $(testid('identity-seed-pinned')).waitForDisplayed({ timeout: 10_000 });
      // ⚠️ The seed is in its field before its person is on the card: the card says whose it is.
      await eventually(
        () => $(testid('identity-card')).getAttribute('data-seed'),
        (seed) => seed === 'devnotes',
        'the person of the sample',
      );
      // France: its phones in a range kept for fiction, a key on every number — nothing marked.
      expect(await $(testid('identity-mark')).isExisting()).toBe(false);
      const first = await eventually(
        iban,
        (text) => /^FR\d{2}( [0-9A-Z]{4}){5} [0-9A-Z]{3}$/.test(text),
        'an IBAN',
      );
      const drawn = await person(first);
      expect(drawn.bank.iban).toBe(first);
      expect(await $(testid('identity-surname')).getText()).toContain(drawn.surname);

      // The card does not contradict itself: the social security number is read off the person.
      const nir = drawn.identifiers.find((identifier) => identifier.kind === 'nir')!.value.replace(/ /g, '');
      expect(nir).toHaveLength(15);
      expect(nir[0]).toBe(drawn.sex === 'M' ? '1' : '2');
      expect(nir.slice(1, 3)).toBe(drawn.birth.date.slice(2, 4));
      expect(nir.slice(3, 5)).toBe(drawn.birth.date.slice(5, 7));

      // A value is copied with a click.
      await $(`[data-field="iban"] ${testid('copy-value')}`).click();
      await eventually(clipboardText, (text) => text === first, 'the IBAN copied');

      // Another seed is another person; the first seed, the first person again.
      await setField(testid('identity-seed'), 'another');
      await eventually(iban, (text) => text !== first, 'someone else');
      await setField(testid('identity-seed'), 'devnotes');
      await eventually(iban, (text) => text === first, 'the same person again');

      // What it generates is what « Luhn et IBAN » verifies: one implementation, asked twice.
      await openTool('checks');
      await setField(testid('checks-input'), first);
      await eventually(
        () => $(testid('checks-verdict')).getAttribute('data-valid'),
        (valid) => valid === 'true',
        'the generated IBAN found valid',
      );
      expect(await $(testid('checks-rib')).getAttribute('data-valid')).toBe('true');
      await $(testid('tool-clear')).click();
    });

    /**
     * A country is a profile: its own numbers under its own names, each read off the person or
     * carrying its key — and a country that has none for a firm says so rather than invent one.
     */
    it('draws a person of another country under that country’s own numbers, and says what a country does not have', async () => {
      interface Drawn {
        country: string;
        sex: string;
        birth: { date: string };
        identifiers: { kind: string; value: string }[];
      }
      const shown = () => $(testid('identity-card')).getAttribute('data-country');
      const choose = async (country: string) => {
        await $(testid('choice-identity-country')).click();
        await $(`${testid('choice-panel-identity-country')} [data-option-id="${country}"]`).click();
        await eventually(shown, (now) => now === country, `a person of ${country}`);
      };
      const value = (field: string) => $(`[data-field="${field}"] ${testid('output-value')}`).getText();
      const named = async () => [
        await $(testid('identity-surname')).getText(),
        await $(testid('identity-given')).getText(),
      ];
      await openTool('identity');
      await $(testid('identity-card')).waitForDisplayed({ timeout: 10_000 });

      // Italy: the codice fiscale is on the card, and says the year, the month and the sex of its bearer.
      await choose('IT');
      await $(testid('identity-copy-json')).click();
      const italian = JSON.parse(
        (await eventually(
          clipboardText,
          (text) => text?.includes('"country": "IT"') === true,
          'an Italian',
        )) ?? '{}',
      ) as Drawn;
      const code = italian.identifiers.find((identifier) => identifier.kind === 'codiceFiscale')!.value;
      expect(await $(`${testid('identity-personal')}[data-kind="codiceFiscale"]`).getText()).toContain(code);
      expect(code).toHaveLength(16);
      expect(code.slice(6, 8)).toBe(italian.birth.date.slice(2, 4));
      expect(code[8]).toBe('ABCDEHLMPRST'[Number(italian.birth.date.slice(5, 7)) - 1]);
      expect(Number(code.slice(9, 11)) > 40).toBe(italian.sex === 'F');
      expect(await value('partitaIva')).toMatch(/^\d{11}$/);
      const iban = await value('iban');
      expect(iban.startsWith('IT')).toBe(true);
      const italianNames = await named();
      // No range is kept for fiction there: its phones are marked as ones that may ring.
      expect(await $(`[data-field="phone"] ${testid('identity-mark')}`).getAttribute('data-mark')).toBe(
        'mayRing',
      );

      // The United States: a routing number where an IBAN would be, and no number with a key for a firm.
      await choose('US');
      // ⚠️ The seed is pinned: the names are the same as in Italy. A name does not follow the country.
      expect(await named()).toEqual(italianNames);
      expect(await $('[data-field="iban"]').isExisting()).toBe(false);
      expect(await value('routingNumber')).toMatch(/^\d{9}$/);
      await $(testid('identity-no-firm-number')).waitForDisplayed({ timeout: 10_000 });
      expect(await $(`${testid('identity-personal')}[data-kind="ssn"]`).getText()).toMatch(/987-65-432\d/);
      expect(await $(`[data-field="phone"] ${testid('identity-mark')}`).isExisting()).toBe(false);
      // A social security number is a format with no key: marked so on the card.
      expect(await $(`${testid('identity-card')} ${testid('identity-mark')}`).getAttribute('data-mark')).toBe(
        'unkeyed',
      );

      // The Italian IBAN is one « Luhn et IBAN » finds valid.
      await openTool('checks');
      await setField(testid('checks-input'), iban);
      await eventually(
        () => $(testid('checks-verdict')).getAttribute('data-valid'),
        (valid) => valid === 'true',
        'the Italian IBAN found valid',
      );
      await $(testid('tool-clear')).click();

      // Vider goes back to France.
      await openTool('identity');
      await $(testid('tool-clear')).click();
      await eventually(shown, (now) => now === 'FR', 'France again');
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
      expect((await $(testid('lorem-counted')).getText()).replace(/\D/g, '')).toBe('5');
    });

    const qrVersion = () => $(testid('qr-code')).getAttribute('data-version');

    it('draws a URL as a QR code, and a larger one at a higher correction level', async () => {
      await openTool('qr');
      await $(testid('tool-clear')).click();
      await $(`${testid('segmented-qr-kind')} [data-segment-id="url"]`).click();
      await $(`${testid('segmented-qr-correction')} [data-segment-id="medium"]`).click();
      await setField(testid('qr-url'), 'https://exemple.fr/doc');

      await eventually(qrVersion, (version) => version === '2', 'a version 2 code');
      expect(await $(testid('qr-facts')).getText()).toMatch(/22\D+26/);
      expect(await $(`${testid('qr-code')} path`).getAttribute('d')).toMatch(/^M4,4h7v1h-7z/);

      await $(`${testid('segmented-qr-correction')} [data-segment-id="high"]`).click();
      await eventually(qrVersion, (version) => version === '3', 'a version 3 code');
    });

    it('warns before a Wi-Fi password is kept as a note, and forgets it once the tool is left', async () => {
      await $(`${testid('segmented-qr-kind')} [data-segment-id="wifi"]`).click();
      await setField(testid('qr-ssid'), 'Maison');
      await setField(testid('qr-password'), 'secret');
      // The URL's code is still drawn until Rust answers: wait for the network's payload, 30 bytes.
      await eventually(
        () => $(testid('qr-code')).getAttribute('data-bytes'),
        (bytes) => bytes === '30',
        'the Wi-Fi network drawn',
      );

      await $(testid('tool-save-as-note')).click();
      await $(testid('save-as-note-warning')).waitForDisplayed({ timeout: 5_000 });
      await $(testid('save-as-note-cancel')).click();
      await $(testid('save-as-note')).waitForExist({ reverse: true, timeout: 5_000 });

      await openTool('lorem');
      await openTool('qr');
      expect(await $(testid('qr-ssid')).getValue()).toBe('Maison');
      expect(await $(testid('qr-password')).getValue()).toBe('');
      await $(testid('tool-clear')).click();
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
      expect(await readEach(testid('dates-group'), '@data-group')).toEqual(['unix', 'formats', 'landmarks']);
      expect(await $(testid('dates-heading')).getText()).toMatch(/2026, \d{2}:\d{2}:20$/);

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

    it('measures a gap on its second tab by the clock, and reads an ISO duration or words', async () => {
      const row = async (attribute: string, id: string) =>
        (await $(`[${attribute}="${id}"] ${testid('output-value')}`).getText()).trim();
      await $(`${testid('segmented-dates-tab')} [data-segment-id="durations"]`).click();

      // Offsets written out: what the clock counts does not depend on the runner's zone.
      await setField(testid('dates-from'), '2026-03-12T08:00:00Z');
      await setField(testid('dates-to'), '2026-10-04T14:41:00Z');
      await eventually(
        () => row('data-elapsed', 'days'),
        (days) => /^206 [jd] 6 h 41$/.test(days),
        'the elapsed days',
      );
      expect(await row('data-elapsed', 'iso')).toMatch(/^P6M22DT\d+H41M$/);

      await setField(testid('dates-duration'), 'PT1H30M');
      await eventually(
        () => row('data-total', 'minutes'),
        (minutes) => minutes === '90',
        'ninety minutes',
      );
      await setField(testid('dates-duration'), '1h30');
      await eventually(
        () => row('data-total', 'iso'),
        (iso) => iso === 'PT1H30M',
        'the ISO form of words',
      );
      await setField(testid('dates-duration'), '90 min');
      await eventually(
        () => row('data-total', 'iso'),
        (iso) => iso === 'PT1H30M',
        'ninety minutes carried into an hour',
      );

      await $(testid('tool-clear')).click();
      await $(`${testid('segmented-dates-tab')} [data-segment-id="convert"]`).click();
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
      // A « no » says when, then: the Monday before, at nine, and the Monday after.
      expect(await $(testid('cron-nearest-before')).getText()).toContain('2026-09-28 09:00:00');
      expect(await $(testid('cron-nearest-after')).getText()).toContain('2026-10-05 09:00:00');
      await setField(testid('cron-check'), '');
    });

    /**
     * « Will this run on Friday afternoon? »: the list was the next ten from now, and the check
     * one exact minute. 2026-10-16 is a Friday, the 17th a Saturday.
     */
    it('counts the runs between two moments: none on a Saturday, seventy-two on a Friday afternoon', async () => {
      const total = () => $(testid('cron-window-count')).getAttribute('data-total');
      await setField(testid('cron-input'), '*/5 * * * 1-5');

      // A Saturday, for an expression of the working days: the answer one came for.
      await setField(testid('cron-from'), '2026-10-17 00:00');
      await setField(testid('cron-until'), '2026-10-18 00:00');
      await eventually(total, (count) => count === '0', 'no run on a Saturday');
      expect(await readEach(testid('cron-run'), 'text')).toEqual([]);

      // A Friday afternoon: counted, and listed by its two ends around how many lie between.
      await setField(testid('cron-from'), '2026-10-16 12:00');
      await setField(testid('cron-until'), '2026-10-16 18:00');
      await eventually(total, (count) => count === '72', 'seventy-two runs on a Friday afternoon');
      const listed = await readEach(testid('cron-run-at'), 'text');
      expect(listed).toHaveLength(10);
      expect(listed[0]).toBe('2026-10-16 12:00:00');
      expect(listed.at(-1)).toBe('2026-10-16 17:55:00');
      expect(await $(testid('cron-window-gap')).getAttribute('data-between')).toBe('62');

      // The shortcut fills both ends with a Friday afternoon of its own: seventy-two again.
      await setField(testid('cron-until'), '');
      await $(testid('cron-window-count')).waitForExist({ reverse: true, timeout: 10_000 });
      await $(`${testid('cron-shortcut')}[data-shortcut="fridayAfternoon"]`).click();
      await eventually(total, (count) => count === '72', 'the Friday afternoon of the shortcut');
      expect(await $(testid('cron-from')).getValue()).toMatch(/^\d{4}-\d{2}-\d{2} 12:00$/);
      expect(await $(testid('cron-until')).getValue()).toMatch(/^\d{4}-\d{2}-\d{2} 18:00$/);

      // An end before its start is said, not counted.
      await setField(testid('cron-until'), '2020-01-01 00:00');
      await eventually(
        () => readEach(testid('cron-window-problem'), '@data-problem'),
        (problems) => problems.join() === 'backwards',
        'the end before the start',
      );
      await setField(testid('cron-from'), '');
      await setField(testid('cron-until'), '');
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
      ).toEqual(['sizes', 'permissions', 'checks', 'cidr']);
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
      expect(await sizeValue('petabyte')).toContain('× 10⁻⁶');
      expect(await readEach(testid('sizes-group'), '@data-group')).toEqual(['bytes', 'decimal', 'binary']);

      await setField(testid('sizes-decimals'), '6');
      await eventually(
        async () => digits(await sizeValue('gibibyte')),
        (gibibytes) => gibibytes === '1396984',
        'six decimals',
      );
      await setField(testid('sizes-decimals'), '3');
    });

    it('estimates a transfer on its second tab, at the rate and by connection', async () => {
      await $(`${testid('segmented-sizes-tab')} [data-segment-id="transfer"]`).click();
      await setField(testid('transfer-size'), '4,7');
      await setField(testid('transfer-rate'), '100');

      await eventually(
        async () => (await $(testid('transfer-time')).getText()).trim(),
        (time) => time === '6 min 58 s',
        'the time at 90 %',
      );
      expect(
        await readEach(`${testid('transfer-connections')} ${testid('output-row')}`, '@data-name'),
      ).toHaveLength(4);

      await $(testid('tool-clear')).click();
      await $(`${testid('segmented-sizes-tab')} [data-segment-id="conversion"]`).click();
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
      await setField(testid('permissions-octal'), '501');
      await eventually(
        () => readEach(testid('permissions-warning'), '@data-warning'),
        (warnings) => warnings.join() === 'othersOverGroup',
        'the group poorer than everybody',
      );
      await $(testid('tool-clear')).click();
    });

    it('checks a card number and an IBAN, in Rust', async () => {
      expect(await bridge.checkDigits({ text: '5555 5555 5555 4444', kind: null })).toMatchObject({
        kind: 'luhn',
        valid: true,
        network: 'mastercard',
      });
      expect(
        await bridge.checkDigits({ text: 'FR15 2004 1010 0505 0001 3M02 606', kind: null }),
      ).toMatchObject({
        kind: 'iban',
        verdict: { kind: 'wrongChecksum', expected: '14' },
      });
    });

    it('says a card number is valid, and forgets it once the tool is left', async () => {
      await openTool('checks');
      await setField(testid('checks-input'), '4111 1111 1111 1111');

      await eventually(
        () => $(testid('checks-verdict')).getAttribute('data-valid'),
        (valid) => valid === 'true',
        'the Luhn verdict',
      );
      expect(await $(testid('checks-network')).getAttribute('data-network')).toBe('visa');

      await $(testid('tools-rail-all')).click();
      await entry('checks').click();
      await $(testid('checks-input')).waitForDisplayed({ timeout: 5_000 });
      expect(await $(testid('checks-input')).getValue()).toBe('');
    });

    it('names an IBAN country and the length it expects', async () => {
      await setField(testid('checks-input'), 'DE89 3704 0044 0532 0130 0');

      await eventually(
        () => $(testid('checks-verdict')).getAttribute('data-problem'),
        (problem) => problem === 'wrongLength',
        'the length refused',
      );
      expect(await $(testid('checks-verdict')).getText()).toContain('22');
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

    const cidrRow = (id: string) => $(`${testid('cidr-facts')} [data-row="${id}"]`).getText();

    it('describes a /21, finds an address inside at its host number, and splits it', async () => {
      await openTool('cidr');
      await $(testid('tool-clear')).click();
      await setField(testid('cidr-network'), '10.24.8.5/21');

      await eventually(
        () => cidrRow('broadcast'),
        (row) => row.includes('10.24.15.255'),
        'the broadcast',
      );
      expect(await $(testid('cidr-normalised')).getText()).toContain('10.24.8.0/21');
      expect(await $(testid('cidr-kind')).getAttribute('data-kind')).toBe('private');
      expect((await cidrRow('usable')).replace(/\D/g, '')).toBe('20462048');

      await setField(testid('cidr-member'), '10.24.12.40');
      await eventually(
        () => $(testid('cidr-membership')).getAttribute('data-inside'),
        (inside) => inside === 'true',
        'the address inside',
      );
      expect((await $(testid('cidr-membership')).getText()).replace(/\D/g, '')).toBe('1064');

      await $(`${testid('cidr-split-choice')}[data-prefix="23"]`).click();
      await eventually(
        () => readEach(testid('cidr-subnet'), '@data-cidr'),
        (subnets) => subnets.join() === '10.24.8.0/23,10.24.10.0/23,10.24.12.0/23,10.24.14.0/23',
        'four /23',
      );
    });

    it('places a plan in the block the largest first, moves to a /22 from the masks, and summarises a list', async () => {
      await setField(testid('cidr-network'), '10.24.8.0/21');
      await setField(testid('cidr-plan'), 'Lien WAN 2\nBureaux 500\nServeurs 60');
      await eventually(
        () => readEach(`${testid('cidr-plan-line')}[data-kind="placed"]`, '@data-cidr'),
        (placed) => placed.join() === '10.24.8.0/23,10.24.10.0/26,10.24.10.64/31',
        'the plan placed',
      );

      await $(`${testid('cidr-mask')}[data-prefix="22"] ${testid('cidr-mask-apply')}`).click();
      await eventually(
        () => $(`${testid('cidr-mask')}[aria-current="true"]`).getAttribute('data-prefix'),
        (prefix) => prefix === '22',
        'the /22 applied',
      );
      expect(await $(testid('cidr-network')).getValue()).toBe('10.24.8.0/22');

      await setField(testid('cidr-list'), '10.24.8.0/24\n10.24.9.0/24\n10.24.13.5 - 10.24.13.20');
      await eventually(
        () => readEach(testid('cidr-summary-block'), 'text'),
        (blocks) =>
          blocks.join() ===
          '10.24.8.0/23,10.24.13.5/32,10.24.13.6/31,10.24.13.8/29,10.24.13.16/30,10.24.13.20/32',
        'the fewest blocks',
      );
      expect(
        await $(`${testid('cidr-summary')} [data-row="supernet"] ${testid('output-value')}`).getText(),
      ).toBe('10.24.8.0/21');
    });

    it('refuses a mask with holes, and reads an IPv6 block', async () => {
      await setField(testid('cidr-network'), '10.0.0.0 255.0.255.0');
      await eventually(
        () => $(testid('cidr-problem')).getAttribute('data-problem'),
        (problem) => problem === 'maskWithHoles',
        'the refusal',
      );

      await setField(testid('cidr-network'), '2001:db8::/32');
      await eventually(
        () => $(testid('cidr-facts')).getAttribute('data-family'),
        (family) => family === 'v6',
        'an IPv6 block',
      );
      expect(await $(testid('cidr-kind')).getAttribute('data-kind')).toBe('documentation');
      await $(testid('tool-clear')).click();
    });
  });

  describe('the references', () => {
    const rows = () => readEach(testid('reference-row'), '@data-key');

    it('sit in a Références panel of their own', async () => {
      await $(testid('tools-rail-all')).click();

      expect(
        await readEach(
          `${testid('tools-panel')}[data-category="reference"] ${testid('tools-entry')}`,
          '@data-tool',
        ),
      ).toEqual([
        'http-status',
        'http-headers',
        'mime-types',
        'ports',
        'signals',
        'ascii',
        'regex-sheet',
        'markdown-sheet',
        'mustache-sheet',
      ]);
    });

    it('opens the category from the rail, and finds a status code by its number', async () => {
      await $(`${testid('tools-rail-category')}[data-category="reference"]`).click();
      await entry('http-status').click();
      await eventually(rows, (keys) => keys.length > 70, 'every code, its words loaded with the tool');

      await setField(testid('reference-search'), '404');

      await eventually(rows, (keys) => keys.join() === '404', 'the search to narrow');
      expect(await $(testid('reference-count')).getText()).toMatch(/1\D+\d{2}/);
    });

    it('finds a code by its words, and says when nothing matches', async () => {
      await setField(testid('reference-search'), 'teapot');
      await eventually(rows, (keys) => keys.join() === '418', 'the teapot');
      expect(
        await $(`[data-key="418"] ${testid('http-status-standing')}`).getAttribute('data-standing'),
      ).toBe('unofficial');

      await setField(testid('reference-search'), 'zzz');
      await $(testid('reference-empty')).waitForDisplayed({ timeout: 5_000 });
    });

    it('keeps nothing as a note and offers no sample, and Vider empties the search', async () => {
      expect(await $(testid('tool-save-as-note')).isExisting()).toBe(false);
      expect(await $(testid('tool-sample')).isExisting()).toBe(false);

      await $(testid('tool-clear')).click();

      await eventually(rows, (keys) => keys.length > 70, 'every code again');
      expect(await $(testid('reference-search')).getValue()).toBe('');
    });

    it('finds the CORS headers by a word, each tagged as a matter of security', async () => {
      await openTool('http-headers');
      await eventually(rows, (keys) => keys.length > 70, 'every header');

      await setField(testid('reference-search'), 'cors');

      await eventually(
        rows,
        (keys) => keys.includes('Access-Control-Allow-Origin') && keys.length < 20,
        'the CORS headers',
      );
      expect(
        await $(`[data-key="Access-Control-Allow-Origin"] ${testid('http-headers-security')}`).isExisting(),
      ).toBe(true);
    });

    it('finds a MIME type by its extension, with or without its dot', async () => {
      await openTool('mime-types');
      await eventually(rows, (keys) => keys.length > 80, 'every type');

      await setField(testid('reference-search'), '.webp');
      await eventually(rows, (keys) => keys.join() === 'image/webp', 'image/webp by .webp');

      await setField(testid('reference-search'), 'json');
      await eventually(rows, (keys) => keys.includes('application/json'), 'application/json by json');
      await $(testid('tool-clear')).click();
    });

    it('finds a port by the start of its number, and a signal by its number exactly', async () => {
      await openTool('ports');
      await eventually(rows, (keys) => keys.length > 60, 'every port');
      await setField(testid('reference-search'), '54');
      await eventually(rows, (keys) => keys.join() === '5432', 'PostgreSQL by 54');
      await $(testid('tool-clear')).click();

      await openTool('signals');
      await eventually(rows, (keys) => keys.length > 30, 'every signal');
      await setField(testid('reference-search'), '9');
      await eventually(rows, (keys) => keys.join() === 'SIGKILL', 'SIGKILL by 9');
      expect(
        await $(`[data-key="SIGKILL"] ${testid('signals-catchable')}`).getAttribute('data-catchable'),
      ).toBe('false');
      await $(testid('tool-clear')).click();
    });

    it('finds a character by its code in any base, or by a name', async () => {
      await openTool('ascii');
      await eventually(rows, (keys) => keys.length === 128, 'the 128 characters');

      await setField(testid('reference-search'), '0x41');
      await eventually(rows, (keys) => keys.join() === '65', 'A by 0x41');

      await setField(testid('reference-search'), 'newline');
      await eventually(rows, (keys) => keys.join() === '10', 'LF by newline');
    });

    it('copies the character itself', async function () {
      await setField(testid('reference-search'), 'A');
      await eventually(rows, (keys) => keys.join() === '65', 'A alone');
      await $(`[data-key="65"] ${testid('copy-value')}`).click();

      // An unreadable clipboard answers null at once, so only the readable case waits.
      const copied = await eventually(
        () => clipboardText(),
        (text) => text === null || text === 'A',
        'the character to reach the clipboard',
      );
      await $(testid('tool-clear')).click();
      if (copied === null) {
        // No readable clipboard on this runner; see `clipboardText`.
        this.skip();
        return;
      }
      expect(copied).toBe('A');
    });

    it('finds a regex construct by its syntax, case and all, with its support in each flavour', async () => {
      await openTool('regex-sheet');
      await eventually(rows, (keys) => keys.length > 60, 'every construct');

      await setField(testid('reference-search'), '(?>');
      await eventually(rows, (keys) => keys.join() === '(?>…)', 'the atomic group');
      expect(
        await $(`${testid('reference-row')} ${testid('regex-sheet-js')}`).getAttribute('data-support'),
      ).toBe('no');

      await setField(testid('reference-search'), 'lookbehind');
      await eventually(rows, (keys) => keys.length === 2, 'both lookbehinds');
      await $(testid('tool-clear')).click();
    });

    it('finds a Markdown construct by its syntax, with where it works', async () => {
      await openTool('markdown-sheet');
      await eventually(rows, (keys) => keys.length > 30, 'every construct');

      await setField(testid('reference-search'), '~~…');
      await eventually(rows, (keys) => keys.join() === '~~…~~', 'strikethrough');
      expect(
        await $(`${testid('reference-row')} ${testid('markdown-sheet-commonmark')}`).getAttribute(
          'data-works',
        ),
      ).toBe('false');
      await $(testid('tool-clear')).click();
    });

    it('shows a Mustache tag with a template, its view and what comes out, braces intact', async () => {
      await openTool('mustache-sheet');
      await eventually(rows, (keys) => keys.length > 10, 'every tag');

      await setField(testid('reference-search'), '{{#');
      await eventually(rows, (keys) => keys.length === 3, 'the three kinds of section');
      // Never through Transloco, which would have emptied every unknown {{name}}.
      expect(
        await $(`[data-key="{{#list}}…{{/list}}"] ${testid('mustache-sheet-example')}`).getText(),
      ).toContain('{{label}}');
      await $(testid('tool-clear')).click();
    });
  });

  describe('comparing two texts, or two documents as values', () => {
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
        formatA: null,
        formatB: null,
        ignoreKeyOrder: true,
        ignoreWhitespace: true,
      });

      expect(answer.kind).toBe('compared');
      if (answer.kind !== 'compared') return;
      expect(answer.counts).toEqual({ added: 2, removed: 1, modified: 4 });
      expect(JSON.parse(answer.patch)).toContainEqual({ op: 'replace', path: '/replicas', value: 6 });
    });

    it('writes the unified diff of two texts from Rust, line endings aside', async () => {
      const answer = await bridge.diffText({
        a: 'host = db\r\nport = 5432\r\n',
        b: 'host = db\nport = 6432\n',
        granularity: 'lines',
        ignoreTrailingWhitespace: false,
        ignoreAllWhitespace: false,
        ignoreCase: false,
        ignoreLineEndings: true,
      });

      expect([answer.added, answer.removed]).toEqual([1, 1]);
      expect(answer.unified).toBe('--- a\n+++ b\n@@ -1,2 +1,2 @@\n host = db\n-port = 5432\n+port = 6432\n');
    });

    it('marks the words that changed inside a line, and shows the unified diff', async () => {
      await openTool('diff');
      await $(`${testid('segmented-diff-mode')} [data-segment-id="text"]`).click();
      await setField(testid('text-diff-a'), 'host = db\nport = 5432\npool = 10\n');
      await setField(testid('text-diff-b'), 'host = db\nport = 6432\npool = 10\nssl = on\n');
      await $(`${testid('segmented-text-diff-granularity')} [data-segment-id="words"]`).click();

      await eventually(
        () => readEach(testid('text-diff-changed'), 'text'),
        (pieces) => pieces.join(',') === '5432,6432',
        'the changed words',
      );
      expect(await readEach(testid('text-diff-row'), '@data-kind')).toEqual([
        'same',
        'modified',
        'same',
        'added',
      ]);

      await $(`${testid('segmented-text-diff-layout')} [data-segment-id="unified"]`).click();
      await $(testid('text-diff-unified')).waitForDisplayed({ timeout: 5_000 });
      expect(await $(testid('text-diff-unified')).getText()).toContain('+ssl = on');
    });

    it('offers to compare two JSON texts as values, and lists the mockup’s seven changes', async () => {
      await setField(testid('text-diff-a'), STAGING);
      await setField(testid('text-diff-b'), PRODUCTION);
      await $(testid('text-diff-as-values')).waitForDisplayed({ timeout: 5_000 });
      await $(testid('text-diff-as-values')).click();

      await eventually(
        () => readEach(testid('structured-diff-change'), '@data-path'),
        (paths) => paths.length === 7,
        'seven changes',
      );
      expect(await readEach(testid('structured-diff-change'), '@data-path')).toEqual([
        '$.replicas',
        '$.database.host',
        '$.database.pool',
        '$.database.readReplica',
        '$.features.betaExports',
        '$.logLevel',
        '$.sentry',
      ]);

      await $(`${testid('structured-diff-change')}[data-path="$.database.pool"]`).click();
      await $(`${testid('structured-diff-row')}.selected`).waitForExist({ timeout: 5_000 });
    });

    it('counts a new key order only when asked', async () => {
      await $(testid('structured-diff-ignore-order')).click();

      await eventually(
        () => readEach(testid('structured-diff-change'), '@data-kind'),
        (kinds) => kinds.includes('reordered'),
        'the reordering',
      );
      await $(testid('structured-diff-ignore-order')).click();
    });

    it('imports a JSON snippet into A, under its title', async () => {
      noteId = (
        await bridge.createNote(
          draft({ spaceId, title: 'config.staging.json', content: STAGING, language: 'json' }),
        )
      ).id;

      await $(testid('structured-diff-open-a')).click();
      await setField(testid('note-picker-search'), 'config.staging');
      await $(`${testid('note-picker-option')}[data-note-id="${noteId}"]`).click();

      await eventually(
        () => readEach(testid('structured-diff-name-a'), 'text'),
        (names) => names[0] === 'config.staging.json',
        'the snippet in A',
      );
    });

    it('compares a YAML with its JSON twin, each read by its shape, by value', async () => {
      await setField(testid('structured-diff-a'), '# the API\nservice: billing\nreplicas: 2\n');
      await setField(testid('structured-diff-b'), '{ "replicas": 3, "service": "billing" }');

      await eventually(
        () => readEach(testid('structured-diff-change'), '@data-path'),
        (paths) => paths.join() === '$.replicas',
        'the one change',
      );
      expect(await readEach(`[data-testid^="structured-diff-read-"]`, '@data-format')).toEqual([
        'yaml',
        'json',
      ]);
      await $(testid('structured-diff-yaml-note')).waitForDisplayed({ timeout: 5_000 });
    });

    it('reads a side in the format forced on it, and says where it stops', async () => {
      await $(testid('choice-structured-diff-format-b')).click();
      await $(`${testid('choice-panel-structured-diff-format-b')} [data-option-id="xml"]`).click();

      await $(testid('structured-diff-unreadable-b')).waitForDisplayed({ timeout: 5_000 });
      expect(await $(testid('structured-diff-unreadable-b')).getText()).toContain('XML');

      await $(testid('choice-structured-diff-format-b')).click();
      await $(`${testid('choice-panel-structured-diff-format-b')} [data-option-id="auto"]`).click();
      await $(testid('structured-diff-unreadable-b')).waitForDisplayed({ timeout: 5_000, reverse: true });
    });

    /**
     * The text tab's sample — an nginx file — left under « Structuré » was announced « lu comme
     * YAML » and answered « 1 modification »: one string changed into another.
     */
    it('says a text is no structure rather than reading it as YAML, and loads the sample of the tab on screen', async () => {
      const tab = (id: string) => $(`${testid('segmented-diff-mode')} [data-segment-id="${id}"]`);
      await tab('text').click();
      await $(testid('tool-sample')).click();
      await eventually(
        () => $(testid('text-diff-a')).getValue(),
        (text) => text.includes('server {'),
        'the sample of the text tab',
      );

      // The same two texts under « Données », which says what it compares: nothing here is a document.
      await tab('structured').click();
      await $(testid('diff-data-line')).waitForDisplayed({ timeout: 5_000 });
      await $(testid('structured-diff-not-structured-a')).waitForDisplayed({ timeout: 10_000 });
      expect(await $(testid('structured-diff-not-structured-b')).isExisting()).toBe(true);
      expect(await $(testid('structured-diff-read-a')).isExisting()).toBe(false);
      expect(await $(testid('structured-diff-modified')).isExisting()).toBe(false);

      // Its button goes back to the text diff, both sides kept.
      await $(testid('structured-diff-as-text-a')).click();
      await $(testid('text-diff-a')).waitForDisplayed({ timeout: 5_000 });
      expect(await $(testid('text-diff-a')).getValue()).toContain('server {');
      expect(await $(testid('text-diff-b')).getValue()).toContain('server {');

      // « Exemple » under « Données » is that tab's own: a YAML against its JSON, one value apart.
      await tab('structured').click();
      await $(testid('tool-sample')).click();
      await eventually(
        async () =>
          [
            ...(await readEach(testid('structured-diff-read-a'), '@data-format')),
            ...(await readEach(testid('structured-diff-read-b'), '@data-format')),
          ].join(),
        (formats) => formats === 'yaml,json',
        'each side of the sample read by its shape',
      );
      expect(await $(testid('structured-diff-a')).getValue()).not.toContain('server {');
      expect(await readEach(testid('structured-diff-change'), '@data-path')).toEqual(['$.version']);
      await $(testid('tool-clear')).click();
    });
  });

  /** Filled through the tool's own fields, so Rust answers it as it would answer typing. */
  describe('a sample', () => {
    async function loadSample(tool: string): Promise<void> {
      await openTool(tool);
      await $(testid('tool-clear')).click();
      await $(testid('tool-sample')).click();
      await $(testid('status-toast')).waitForDisplayed({ timeout: 5_000 });
    }

    it('decodes a made-up token and verifies it with its made-up key', async () => {
      await loadSample('jwt');

      await eventually(
        () => $(testid('jwt-payload')).getText(),
        (payload) => payload.includes('Ada Lovelace'),
        'the sample token decoded',
      );
    });

    it('checks the published example IBAN', async () => {
      await loadSample('checks');

      await eventually(
        () => $(testid('checks-verdict')).getAttribute('data-valid'),
        (valid) => valid === 'true',
        'the sample IBAN found valid',
      );
      expect(await $(testid('checks-rib')).getAttribute('data-valid')).toBe('true');
      // A card number or an IBAN never ends up in a note in clear.
      expect(await $(testid('tool-save-as-note')).getAttribute('aria-disabled')).toBe('true');
    });

    it('converts its document, which replaces what was typed', async () => {
      await openTool('convert');
      await setField(testid('convert-input'), '{"typed": true}');
      await loadSample('convert');

      await eventually(
        () => $(testid('convert-output')).getText(),
        (converted) => converted.includes('nom = "Dupont"') && !converted.includes('typed'),
        'the sample converted',
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
      await openTool('case');
      await $(testid('tool-clear')).click();

      await eventually(
        () => $(testid('tool-save-as-note')).getAttribute('aria-disabled'),
        (disabled) => disabled === 'true',
        'nothing to keep',
      );
    });

    it('goes where the dialog puts it, with its tags, as the tool shaped it', async () => {
      await setField(testid('case-input'), 'Été 2026 !');
      await eventually(
        () => $(`${testid('output-row')}[data-name="Slug"] ${testid('output-value')}`).getText(),
        (slug) => slug === 'ete-2026',
        'the slug',
      );

      await $(testid('tool-save-as-note')).click();
      await $(testid('save-as-note')).waitForDisplayed({ timeout: 5_000 });
      await setField(testid('save-as-note-title'), 'Slug de la page été');
      await pickChoice('save-as-note-space', spaceId);
      await setField(testid('save-as-note-tags'), 'web, seo');
      await $(testid('save-as-note-submit')).click();
      await $(testid('save-as-note')).waitForExist({ reverse: true, timeout: 10_000 });

      const [note] = await inSpace();
      expect(note).toMatchObject({ title: 'Slug de la page été', language: 'txt', kind: 'snippet' });
      // The tool shaped it: every case named, the slug last.
      const body = (await bridge.getNote(note!.id)).content;
      expect(body).toMatch(/^snake_case +ete_2026$/m);
      expect(body).toMatch(/^slug +ete-2026$/m);
      expect([...(note?.tags ?? [])].sort()).toEqual(['seo', 'web']);
      expect(note?.source).toContain('/');
    });
  });
});
