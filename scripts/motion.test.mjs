import assert from 'node:assert/strict';
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, sep } from 'node:path';
import { describe, it } from 'node:test';

/**
 * ⚠️ `node --test` and not a `*.spec.ts`, for the same reason the palette tests are here:
 * this reads the shipped stylesheets off disk, and the Angular builder compiles its specs
 * for a browser, where `node:fs` does not exist.
 *
 * What it holds is the one thing motion can quietly lose: a duration written out in a
 * component is a duration `prefers-reduced-motion` still reaches — it is a blanket rule —
 * but it is also one nobody can retune, and the two that mattered had already drifted to
 * three different values (120ms, 120ms, 0.15s) before there was any motion to speak of.
 */
const ROOT = 'src';
const GLOBAL = 'src/styles/styles.scss';

/** ⚠️ Separators normalised: `join` answers backslashes on Windows and CI runs on Linux. */
function stylesheets(directory) {
  return readdirSync(directory).flatMap((entry) => {
    const path = join(directory, entry).split(sep).join('/');
    if (statSync(path).isDirectory()) return stylesheets(path);
    return path.endsWith('.scss') ? [path] : [];
  });
}

const SHEETS = stylesheets(ROOT).map((path) => ({ path, source: readFileSync(path, 'utf8') }));

/** A bare duration: `120ms`, `0.15s`. The variables read `var(--motion…)` instead. */
const BARE_DURATION = /(?:^|[\s:,(])\d*\.?\d+m?s\b/;

/** Comments say "120ms" as prose; only declarations are the subject here. */
function declarations(source) {
  return source
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => !line.startsWith('//') && !line.startsWith('*') && !line.startsWith('/*'));
}

describe('motion', () => {
  it('declares its two durations once, as variables', () => {
    const global = readFileSync(GLOBAL, 'utf8');

    assert.match(global, /--motion-fast:\s*\d+ms;/);
    assert.match(global, /--motion:\s*\d+ms;/);
  });

  /** ⚠️ A duration written out is one nobody can retune, and the three that existed had
   *  already drifted to three different values. */
  it('writes no duration of its own anywhere', () => {
    const offenders = SHEETS.flatMap(({ path, source }) =>
      declarations(source)
        .filter((line) => /\b(transition|animation)(-duration|-delay)?\s*:/.test(line))
        .filter((line) => BARE_DURATION.test(line))
        .map((line) => `${path}: ${line}`),
    ).filter((entry) => !entry.startsWith(`${GLOBAL}:`));

    assert.deepEqual(offenders, []);
  });

  /**
   * ⚠️ `all` catches layout properties too, and a transition on one of those is what makes
   * a list feel heavy — the canvas renders every note with no virtualisation.
   */
  it('never transitions everything', () => {
    const offenders = SHEETS.flatMap(({ path, source }) =>
      declarations(source)
        .filter((line) => /transition(-property)?\s*:\s*all\b/.test(line))
        .map((line) => `${path}: ${line}`),
    );

    assert.deepEqual(offenders, []);
  });

  /**
   * ⚠️ One block, and it has to stay the only one: it is what turns every duration above
   * off without naming any of them. A second copy somewhere else is a rule that stops
   * covering whatever is added next.
   */
  it('turns all of it off in exactly one place', () => {
    const blocks = SHEETS.filter(({ source }) => source.includes('prefers-reduced-motion'));

    assert.deepEqual(
      blocks.map(({ path }) => path),
      [GLOBAL],
    );
  });
});
