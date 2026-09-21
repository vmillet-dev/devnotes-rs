import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { describe, it } from 'node:test';

/**
 * ⚠️ `node --test` and not a `*.spec.ts`, like the palette and the sleep sweep: this reads
 * the shipped stylesheets off disk, and the Angular builder compiles its specs for a
 * browser, where `node:fs` does not exist.
 *
 * The linter catches most of the accessibility rules and cannot catch this one: a control
 * with no `:focus-visible` is valid CSS and valid HTML, and the keyboard simply walks it
 * invisibly. Four of them had accumulated, the tag rail among them (#233).
 */
const ROOT = 'src/app';

function stylesheets(directory) {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) return stylesheets(path);
    return entry.name.endsWith('.component.scss') ? [path] : [];
  });
}

/**
 * What marks a stylesheet as drawing something the keyboard can reach. ⚠️ `cursor:
 * pointer` and not a `<button>` count: the templates are elsewhere, and a control is
 * styled where it is drawn.
 */
function drawsAControl(source) {
  return /cursor:\s*pointer/.test(source);
}

/** Either written out or taken from the mixin — both end up as the same declaration. */
function saysWhereTheKeyboardIs(source) {
  return source.includes(':focus-visible') || source.includes('focus-ring');
}

describe('every control says where the keyboard is', () => {
  const sheets = stylesheets(ROOT).filter((path) => drawsAControl(readFileSync(path, 'utf8')));

  it('finds the stylesheets to sweep', () => {
    assert.ok(sheets.length > 20, `only ${sheets.length} stylesheets draw a control`);
  });

  for (const path of sheets) {
    it(path, () => {
      assert.ok(
        saysWhereTheKeyboardIs(readFileSync(path, 'utf8')),
        `${path} draws a control and no focus ring: add \`@include focus-ring;\``,
      );
    });
  }
});

/**
 * The other half of the same omission. A ring nobody drew and a ring nobody left room for
 * look the same to the keyboard, and both are valid CSS.
 *
 * ⚠️ A box that scrolls on one axis scrolls on the other: `overflow-x: auto` makes the
 * used value of `overflow-y` `auto` too — there is no scrolling sideways while
 * overflowing upwards — so a horizontal rail of pills clips their rings flat against its
 * own edges. The tag rail did exactly that (#279).
 */
const ROOM = Number(
  /@mixin ring-room\(\$room: (\d+)px\)/.exec(readFileSync('src/styles/_mixins.scss', 'utf8'))?.[1],
);

/** The `{ … }` the declaration at `at` sits in, nested blocks and all. */
function blockAround(source, at) {
  let depth = 0;
  let open = -1;
  for (let index = at; index >= 0; index -= 1) {
    if (source[index] === '}') depth += 1;
    else if (source[index] === '{') {
      if (depth === 0) {
        open = index;
        break;
      }
      depth -= 1;
    }
  }
  if (open < 0) return '';

  depth = 0;
  for (let index = open; index < source.length; index += 1) {
    if (source[index] === '{') depth += 1;
    else if (source[index] === '}') {
      depth -= 1;
      if (depth === 0) return source.slice(open, index);
    }
  }
  return source.slice(open);
}

/** Block-axis padding: the longhand if there is one, else the shorthand's first value. */
function blockPadding(block) {
  const longhand = /padding-(?:block|top):\s*(\d+(?:\.\d+)?)px/.exec(block);
  if (longhand) return Number(longhand[1]);

  const shorthand = /padding:\s*([^;]+);/.exec(block);
  if (!shorthand) return 0;

  const values = shorthand[1].trim().split(/\s+/);
  const top = /^(\d+(?:\.\d+)?)px$/.exec(values[0] ?? '');
  return top ? Number(top[1]) : 0;
}

describe('a row that scrolls leaves room for the ring', () => {
  const scrollers = stylesheets(ROOT).flatMap((path) => {
    const source = readFileSync(path, 'utf8');
    return [...source.matchAll(/overflow-x:\s*(?:auto|scroll)/g)].map((match) => ({
      path,
      block: blockAround(source, match.index),
    }));
  });

  it('reads the room out of the mixin', () => {
    assert.ok(ROOM >= 4, `ring-room defaults to ${ROOM}px, which is thinner than the ring itself`);
  });

  it('finds the rows that scroll', () => {
    assert.ok(scrollers.length >= 2, `only ${scrollers.length} rows scroll sideways`);
  });

  for (const { path, block } of scrollers) {
    it(path, () => {
      assert.ok(
        block.includes('@include ring-room') || blockPadding(block) >= ROOM,
        `${path} scrolls sideways and clips the focus ring of whatever is in it: add \`@include ring-room;\``,
      );
    });
  }
});
