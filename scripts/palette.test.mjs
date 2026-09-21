import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { describe, it } from 'node:test';

/**
 * ⚠️ `node --test` and not a `*.spec.ts`, for the same reason the release-notes tests are
 * here: this reads a shipped file off disk, and the Angular builder compiles its specs for
 * a browser, where `node:fs` does not exist and `?raw` has no loader. Both were tried.
 */
const STYLESHEET = readFileSync('src/styles/styles.scss', 'utf8');

/** WCAG 2.2 AA for text below 18.66px, which is what every one of these is drawn at. */
const AA = 4.5;

/** WCAG 2.2 AA (1.4.11) for the boundary of a control, and for a focus ring. */
const AA_UI = 3;

/**
 * The four plain surfaces. ⚠️ Not the only backgrounds text lands on — a badge draws its
 * label on a **tint** of one of the hues, which is measured separately below.
 */
const SURFACES = ['--bg-0', '--bg-1', '--bg-2', '--bg-3'];

/** Drawn as text somewhere, so each has to clear AA on every plain surface above. */
const TEXT = ['--text-0', '--text-1', '--text-2', '--amber-text', '--green', '--blue', '--red', '--purple'];

/**
 * Drawn as a line the eye has to find — a focus ring, a selected chip's border — and never
 * as a word. A lower bar than text, and a real one: the accent is bright enough to be a
 * surface precisely because it is not asked to be read.
 */
const EDGES = ['--amber-edge'];

/**
 * Never text and never a line it matters to see: hairlines, the softer accent behind a
 * hover, the accent as a **surface**, the ink drawn *on* that surface, and the titlebar's
 * decorative dots. ⚠️ Listed rather than skipped, so a colour added to the palette fails
 * the last test here until somebody says which of the four it is.
 */
const NOT_TEXT = [
  '--line',
  '--line-soft',
  '--line-no',
  '--amber-fill',
  '--amber-dim',
  '--amber-ink',
  '--dot-red',
  '--dot-yellow',
  '--dot-green',
];

/**
 * The dark palette is the bare `:root`, deliberately — see the comment above it.
 *
 * ⚠️ A declaration may hand down another variable rather than a hex: on a dark surface one
 * amber does every job, so two of the three roles are declared as the third. Resolved one
 * level, which is all the file uses.
 */
function block(theme) {
  const opening = theme === 'dark' ? ':root {' : ":root[data-theme='light'] {";
  const start = STYLESHEET.indexOf(opening);
  assert.ok(start >= 0, `no ${theme} block in styles.scss`);

  const body = STYLESHEET.slice(start + opening.length, STYLESHEET.indexOf('\n}', start));
  const declared = Object.fromEntries(
    [...body.matchAll(/(--[a-z0-9-]+):\s*(#[0-9a-f]{6}|var\(--[a-z0-9-]+\))/g)].map((m) => [m[1], m[2]]),
  );

  for (const [name, value] of Object.entries(declared)) {
    const handed = /^var\((--[a-z0-9-]+)\)$/.exec(value);
    if (!handed) continue;

    const resolved = declared[handed[1]];
    assert.ok(resolved?.startsWith('#'), `${name} hands down ${handed[1]}, which is not a colour`);
    declared[name] = resolved;
  }

  return declared;
}

/**
 * The strongest tint any badge is drawn on. ⚠️ It is `tint-badge`'s own default, and the
 * highest alpha in use at a call site — a stronger one typed later would want this raised
 * with it.
 */
const MAX_TINT = 0.15;

/** The hues a badge tints its background with, and the label it then draws on that tint. */
const TINTED = ['--amber-fill', '--green', '--blue', '--red', '--purple', '--text-1'];
const BADGE_INK = '--text-1';

function luminance(hex) {
  const value = Number.parseInt(hex.slice(1), 16);
  const channels = [(value >> 16) & 255, (value >> 8) & 255, value & 255].map((raw) => {
    const part = raw / 255;
    return part <= 0.03928 ? part / 12.92 : ((part + 0.055) / 1.055) ** 2.4;
  });

  return 0.2126 * channels[0] + 0.7152 * channels[1] + 0.0722 * channels[2];
}

/** What a translucent fill actually composites to over an opaque surface. */
function mix(hex, over, alpha) {
  const channels = (value) => {
    const n = Number.parseInt(value.slice(1), 16);
    return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
  };
  const [front, back] = [channels(hex), channels(over)];
  const blended = front.map((value, index) => Math.round(value * alpha + back[index] * (1 - alpha)));

  return '#' + blended.map((value) => value.toString(16).padStart(2, '0')).join('');
}

function contrast(a, b) {
  const [lighter, darker] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (lighter + 0.05) / (darker + 0.05);
}

for (const theme of ['dark', 'light']) {
  describe(`the ${theme} palette`, () => {
    const declared = block(theme);
    // ⚠️ The light block redefines only what changes, so what it does not name it inherits
    // from the dark one — reading it from the wrong block would test a colour twice and
    // check another never.
    const of = (name) => declared[name] ?? block('dark')[name];

    it('draws every text colour legibly on every plain surface', () => {
      for (const name of TEXT) {
        for (const surface of SURFACES) {
          const ratio = contrast(of(name), of(surface));

          assert.ok(
            ratio >= AA,
            `${name} (${of(name)}) on ${surface} (${of(surface)}) is ${ratio.toFixed(2)}:1, AA asks ${AA}:1`,
          );
        }
      }
    });

    /**
     * ⚠️ A tint moves the background **toward** the colour it is made of, so a badge's label
     * has less to work with than the plain surface underneath suggests. The hue used to be
     * the label as well, which cost about a point and put light amber at 4.20:1.
     */
    it('draws a badge label legibly on the tint it sits on', () => {
      for (const hue of TINTED) {
        for (const surface of SURFACES) {
          const tinted = mix(of(hue), of(surface), MAX_TINT);
          const ratio = contrast(of(BADGE_INK), tinted);

          assert.ok(
            ratio >= AA,
            `${BADGE_INK} on a ${MAX_TINT} ${hue} tint over ${surface} is ${ratio.toFixed(2)}:1, AA asks ${AA}:1`,
          );
        }
      }
    });

    /**
     * ⚠️ The bar a line has to clear, and the reason the accent can be bright at all: the
     * fill is a surface, so it is only ever read *through* the ink on it, while the ring
     * has to be findable against the four surfaces it is drawn over.
     */
    it('draws every edge colour findably on every plain surface', () => {
      for (const name of EDGES) {
        for (const surface of SURFACES) {
          const ratio = contrast(of(name), of(surface));

          assert.ok(
            ratio >= AA_UI,
            `${name} (${of(name)}) on ${surface} (${of(surface)}) is ${ratio.toFixed(2)}:1, AA asks ${AA_UI}:1`,
          );
        }
      }
    });

    /** The one pairing that is not text on a surface: the label inside the amber fill. */
    it('draws the ink legibly on solid amber', () => {
      const ratio = contrast(of('--amber-ink'), of('--amber-fill'));

      assert.ok(ratio >= AA, `the ink on amber is ${ratio.toFixed(2)}:1`);
    });

    /**
     * ⚠️ What stops the two lists above going quietly out of date. `--text-2` was under AA
     * on all four surfaces for as long as it existed, across 129 declarations, and nothing
     * anywhere said so.
     */
    it('has a verdict on every colour it declares', () => {
      const classified = new Set([...SURFACES, ...TEXT, ...EDGES, ...NOT_TEXT]);
      const unclassified = Object.keys(declared).filter((name) => !classified.has(name));

      assert.deepEqual(
        unclassified,
        [],
        'each of these is text or it is not — say which in palette.test.mjs',
      );
    });
  });
}

/**
 * ⚠️ The promise the three-way split was made on: the accent's **surface** is one colour
 * for the whole application, and the dark theme does not move. Only the line and the label
 * step down on a light background, and only in the light block.
 */
describe('the accent surface', () => {
  it('is declared once, and is the same colour in both themes', () => {
    const declarations = [...STYLESHEET.matchAll(/--amber-fill:/g)];

    assert.equal(declarations.length, 1, 'the light theme must inherit the fill, not redefine it');
  });

  it('is what both other roles are in the dark theme', () => {
    const dark = block('dark');

    assert.equal(dark['--amber-edge'], dark['--amber-fill']);
    assert.equal(dark['--amber-text'], dark['--amber-fill']);
  });
});
