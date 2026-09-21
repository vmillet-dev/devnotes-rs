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
