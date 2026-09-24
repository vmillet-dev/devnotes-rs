import assert from 'node:assert/strict';
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, it } from 'node:test';

/**
 * `node --test` and not a `*.spec.ts`, like the palette file beside it: this reads the
 * shipped spec files off disk, and the Angular builder compiles its own specs for a browser.
 */
const SPECS = 'e2e/specs';

/** How far after a `browser.pause` something still counts as waiting on it. */
const REACH = 4;

/** The reach of the comment that excuses one, which sits directly above. */
const EXCUSE_REACH = 3;

/**
 * The lookahead: not a literal `expect(` on the very next line, which misses a value read now
 * and asserted two lines down, and a blank line between the pause and the assertion.
 */
const READS = /expect\(|const .* = await|await (bridge|canvas|editor|board|crumb|trash|settings)\./;

/**
 * The one exception, and it has to say so: an assertion that **nothing** happened cannot
 * wait on a condition, because nothing arriving is not one. Everything else waits through
 * `eventually` in `e2e/support/app.ts`.
 */
const DELIBERATE = /deliberately/;

function sleepingAssertions() {
  const found = [];

  for (const name of readdirSync(SPECS).filter((file) => file.endsWith('.e2e.ts'))) {
    const lines = readFileSync(join(SPECS, name), 'utf8').split('\n');

    lines.forEach((line, index) => {
      if (!line.includes('browser.pause')) return;

      const after = lines.slice(index + 1, index + 1 + REACH).join('\n');
      const above = lines.slice(Math.max(0, index - EXCUSE_REACH), index).join('\n');
      if (READS.test(after) && !DELIBERATE.test(above)) {
        found.push(`${name}:${index + 1}`);
      }
    });
  }

  return found;
}

describe('a scenario waits on a condition, not on a duration', () => {
  it('has no assertion sleeping behind a browser.pause', () => {
    const sleeping = sleepingAssertions();

    assert.deepEqual(
      sleeping,
      [],
      `these wait on a duration before reading or asserting:\n  ${sleeping.join('\n  ')}\n` +
        'Use `eventually(read, matches, what)`, or say in a comment above why the wait is deliberate.',
    );
  });

  it('finds the specs at all, so a moved directory cannot pass by finding nothing', () => {
    const specs = readdirSync(SPECS).filter((file) => file.endsWith('.e2e.ts'));

    assert.ok(specs.length > 15, `expected the whole suite under ${SPECS}, saw ${specs.length}`);
  });
});
