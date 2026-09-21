import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { describe, it } from 'node:test';

/**
 * The board is laid out twice: once in Rust, which decides how tall a zone has to be for
 * what is filed into it, and once by the browser, which actually flows the cards. Neither
 * side can see the other, both are valid on their own, and the only symptom of a
 * disagreement is a band of empty board under the cards — or a card behind a scrollbar.
 *
 * ⚠️ That is exactly how #284 happened: `columns_in` took a scrollbar off the width that
 * `.zone-body` was not showing, said one column where the browser flowed two, and bought
 * a whole extra row for the next card filed in. No type catches it and no test did.
 *
 * ⚠️ `node --test` and not a `*.spec.ts`, like the other sweeps: this reads the shipped
 * stylesheets and the shipped Rust off disk, and the Angular builder compiles its specs
 * for a browser, where `node:fs` does not exist.
 *
 * What is **not** here, and cannot be: `ZONE_HEADER`. The header's height comes from its
 * padding plus whatever the text lands on, and no stylesheet states it.
 */
const BOARD_RS = 'src-tauri/src/folders/board.rs';
const MIXINS = 'src/styles/_mixins.scss';
const ZONE_SCSS = 'src/app/notes/canvas/board/board-zone/board-zone.component.scss';
const BOARD_SCSS = 'src/app/notes/canvas/board/board.component.scss';
const CARD_SCSS = 'src/app/notes/canvas/note-card/note-card.component.scss';
const GESTURE_TS = 'src/app/notes/canvas/board/board-gesture.ts';

function read(path) {
  return readFileSync(path, 'utf8');
}

/** One number out of one file, or a failure naming the pattern that stopped matching. */
function only(path, pattern) {
  const found = pattern.exec(read(path));
  assert.ok(found, `${path} no longer matches ${pattern} — the sweep cannot read it any more`);

  return Number(found[1]);
}

function rustConst(name) {
  return only(BOARD_RS, new RegExp(`pub const ${name}: i32 = (\\d+);`));
}

/** The `{ … }` of one rule, so a number is read from the block that declares it. */
function block(path, selector) {
  const source = read(path);
  const at = source.indexOf(`${selector} {`);
  assert.ok(at >= 0, `${path} no longer has a \`${selector}\` rule`);

  const end = source.indexOf('\n}', at);
  return source.slice(at, end < 0 ? undefined : end);
}

function inBlock(path, selector, pattern) {
  const found = pattern.exec(block(path, selector));
  assert.ok(found, `\`${selector}\` in ${path} no longer matches ${pattern}`);

  return Number(found[1]);
}

describe('the board is measured the same on both sides', () => {
  it('agrees on how wide a card is', () => {
    assert.equal(inBlock(MIXINS, '@mixin board-card', /width:\s*(\d+)px/), rustConst('CARD_WIDTH'));
  });

  /** ⚠️ Fixed, not nominal: the zone's height is computed from this number of rows. */
  it('agrees on how tall a card is', () => {
    assert.equal(inBlock(CARD_SCSS, '.card', /height:\s*(\d+)px/), rustConst('CARD_HEIGHT'));
  });

  it('agrees on the gap between two cards', () => {
    assert.equal(inBlock(ZONE_SCSS, '.zone-body', /gap:\s*(\d+)px/), rustConst('GAP'));
  });

  it('agrees on the padding inside a zone', () => {
    assert.equal(inBlock(ZONE_SCSS, '.zone-body', /padding:\s*(\d+)px/), rustConst('ZONE_PADDING'));
  });

  /** ⚠️ `box-sizing` is `border-box`, so the hairline comes off the width Rust was given. */
  it('agrees on the zone hairline the width has to pay for', () => {
    assert.equal(inBlock(ZONE_SCSS, '.zone', /border:\s*(\d+)px solid/), rustConst('ZONE_BORDER'));
  });

  /**
   * The count and the flow, written out: whatever `columns_in` answers has to be what fits
   * between the hairlines and the padding, and never one more.
   */
  it('counts the columns the browser will actually flow', () => {
    const card = rustConst('CARD_WIDTH');
    const gap = rustConst('GAP');
    const chrome = (rustConst('ZONE_BORDER') + rustConst('ZONE_PADDING')) * 2;
    const formula = /let inner = width - ZONE_BORDER \* 2 - ZONE_PADDING \* 2;/;

    assert.match(read(BOARD_RS), formula, 'columns_in no longer measures the zone the browser way');

    // The two widths either side of the turn, as the flow sees them.
    const two = chrome + card * 2 + gap;
    assert.ok(two - chrome >= card * 2 + gap, 'two cards and their gap must fit in what is left');
    assert.ok(two - 1 - chrome < card * 2 + gap, 'one pixel less must not');
  });

  /**
   * ⚠️ And to the height, which is the half that bites hardest: `.zone-body` is the box
   * that scrolls, so a zone one pixel short of its rows shows a vertical scrollbar, the
   * scrollbar takes a slice of the row, and the row wraps. Two across become one.
   */
  it('pays for the hairlines in the height as well as the width', () => {
    assert.match(
      read(BOARD_RS),
      /pub fn zone_height\([^)]*\)[^}]*ZONE_BORDER \* 2/s,
      "zone_height no longer leaves room for the zone's own hairlines",
    );
  });

  /** ⚠️ The dotted lattice every gesture snaps to: drift and a snapped board stops looking it. */
  it('snaps to the grid it draws', () => {
    const drawn = inBlock(BOARD_SCSS, '.board', /background-size:\s*(\d+)px/);

    assert.equal(drawn, only(GESTURE_TS, /export const GRID_PX = (\d+);/));
  });
});
