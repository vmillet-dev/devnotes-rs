import { expect } from '@wdio/globals';
import { readFileSync } from 'node:fs';

import { canvas } from '../pageobjects/canvas.page.js';
import { PASSPHRASE } from '../support/app.js';
import { bridge, query } from '../support/bridge.js';
import { vaultPath } from '../support/profile.js';

/**
 * Last on purpose: it leaves the profile behind a phrase nothing else in the run
 * knows. Nothing after it unlocks — the process stays open for the whole suite — but a
 * scenario inserted after this one would be the first to find out the hard way.
 *
 * What it proves is the shape of the change: the key file is rewritten, the notes are
 * not, and the session carries on. Read from Node, against the file rather than against
 * what the application says about it.
 */
describe('Changing the passphrase', () => {
  const NEXT = 'a second end-to-end passphrase';

  function keyFile(): { kdf: { salt: string }; key: string } {
    return JSON.parse(readFileSync(vaultPath(), 'utf8')) as { kdf: { salt: string }; key: string };
  }

  before(canvas.open);

  it('refuses a change that cannot name the current phrase, and rewrites nothing', async () => {
    const before = keyFile();

    const refused = await bridge.changePassphrase('not the current one', NEXT).then(
      () => 'it was not refused',
      (error: Error) => error.message,
    );

    expect(refused).toContain('wrongPassphrase');
    expect(keyFile()).toEqual(before);
  });

  it('rewraps the key without touching a note', async () => {
    const before = keyFile();
    const corpus = (await bridge.queryNotes(query())).matched;
    expect(corpus).toBeGreaterThan(0);

    await bridge.changePassphrase(PASSPHRASE, NEXT);

    const after = keyFile();
    // A fresh salt and a fresh wrapping, so neither phrase says anything about the other.
    expect(after.kdf.salt).not.toBe(before.kdf.salt);
    expect(after.key).not.toBe(before.key);

    // The library is still open on the same key: a change that re-encrypted would have
    // had to stop and restart everything to prove as much.
    expect((await bridge.queryNotes(query())).matched).toBe(corpus);
  });
});
