import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { describe, it } from 'node:test';

/**
 * The site draws with the application's colours, copied: this holds the copy to
 * `crates/ui/src/theme/tokens.rs`, where the application's palette is written. A colour changed
 * there and not here, or the other way round, fails here.
 */
const RUST = readFileSync('../crates/ui/src/theme/tokens.rs', 'utf8');
const CSS = readFileSync('src/styles/_tokens.scss', 'utf8');

/** `bg_0: hex(0x15171c)` → `--bg-0: #15171c`; `amber_fill: AMBER` follows the constant. */
function rustPalette(name) {
  const start = RUST.indexOf(`pub const ${name}: Palette = Palette {`);
  assert.ok(start >= 0, `no ${name} palette in tokens.rs`);
  const body = RUST.slice(start, RUST.indexOf('\n};', start));
  const amber = /const AMBER: Color32 = hex\(0x([0-9a-f]{6})\)/.exec(RUST)?.[1];
  return Object.fromEntries(
    [...body.matchAll(/^\s+([a-z0-9_]+): (?:hex\(0x([0-9a-f]{6})\)|(AMBER)),/gm)].map((m) => [
      `--${m[1].replaceAll('_', '-')}`,
      `#${m[2] ?? amber}`,
    ]),
  );
}

/** The hex colours of one CSS block; a variable handed down (`var(--amber-fill)`) is resolved. */
function cssPalette(opening) {
  const start = CSS.indexOf(opening);
  assert.ok(start >= 0, `no ${opening} block in _tokens.scss`);
  const body = CSS.slice(start + opening.length, CSS.indexOf('\n}', start));
  const declared = Object.fromEntries(
    [...body.matchAll(/(--[a-z0-9-]+):\s*(#[0-9a-f]{6}|var\(--[a-z0-9-]+\))/g)].map((m) => [m[1], m[2]]),
  );
  return Object.fromEntries(
    Object.entries(declared).map(([name, value]) => {
      const handed = /^var\((--[a-z0-9-]+)\)$/.exec(value);
      return [name, handed ? declared[handed[1]] : value];
    }),
  );
}

describe('tokens', () => {
  it('holds the dark palette of the application', () => {
    const css = cssPalette(':root {');
    for (const [name, hex] of Object.entries(rustPalette('DARK'))) {
      if (name !== '--backdrop') assert.equal(css[name], hex, name);
    }
  });

  it('holds the light palette of the application', () => {
    // The light block redefines what a light surface needs; the rest is the dark one's.
    const css = { ...cssPalette(':root {'), ...cssPalette(":root[data-theme='light'] {") };
    for (const [name, hex] of Object.entries(rustPalette('LIGHT'))) {
      if (name !== '--backdrop') assert.equal(css[name], hex, name);
    }
  });
});
