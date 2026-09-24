import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { describe, it } from 'node:test';

/**
 * `node --test` and not a `*.spec.ts`, like the palette and the focus-ring sweeps: this
 * reads two shipped files off disk, and the Angular builder compiles its specs for a
 * browser, where `node:fs` does not exist.
 *
 * A format with no rule is valid CSS and valid HTML. Its badge is simply bare text — no
 * fill, no hairline, and the label in whatever colour surrounds it. Six of the nineteen
 * were in that state, and nothing anywhere said so.
 */
const ENUM = readFileSync('src-tauri/src/notes/language.rs', 'utf8');
const STYLESHEET = readFileSync('src/app/notes/ui/language-badge/language-badge.component.scss', 'utf8');

/**
 * The spelling each variant carries in `closed_enum!`, which is the single one: serde, the
 * column, `Display` and `FromStr` all read it, and so does the `lang-` class.
 */
function languages() {
  const body = ENUM.slice(ENUM.indexOf('pub enum Language {'), ENUM.indexOf('\n}'));
  return [...body.matchAll(/=\s*"([a-z0-9]+)"/g)].map((match) => match[1]);
}

function hued() {
  return new Set([...STYLESHEET.matchAll(/\.lang-([a-z0-9]+)\b/g)].map((match) => match[1]));
}

describe('every format draws a badge', () => {
  const all = languages();

  it('reads the enum it is meant to cover', () => {
    assert.ok(all.length >= 19, `only found ${all.length} languages — has closed_enum! moved?`);
    assert.ok(all.includes('txt'), 'the default is missing, so the parse is wrong');
  });

  for (const language of all) {
    it(language, () => {
      assert.ok(hued().has(language), `"${language}" has no .lang-${language} rule: its badge is bare text`);
    });
  }

  /** The other direction: a rule for a format that no longer exists is dead weight. */
  it('has no rule for a format the enum does not name', () => {
    const orphans = [...hued()].filter((name) => name !== 'tag' && !all.includes(name));

    assert.deepEqual(orphans, [], 'these .lang-* rules match no member of Language');
  });
});
