import { browser, expect } from '@wdio/globals';
import { existsSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { canvas } from '../pageobjects/canvas.page.js';
import { fileMenu } from '../pageobjects/titlebar.page.js';
import { press, reloadCanvas } from '../support/app.js';
import { bridge, draft, homeSpaceId, query } from '../support/bridge.js';

/**
 * Every operation reports, including when it changed nothing — the one case
 * indistinguishable from a failure without a report.
 *
 * The OS file picker is not driven here (see `support/app.ts`): the commands take a
 * path, and the path is where the real work happens.
 */
/** The message rather than the throw: a refusal is what these two assertions are about. */
async function failureOf(running: Promise<unknown>): Promise<string> {
  return running.then(
    () => 'it was not refused',
    (error: Error) => error.message,
  );
}
describe('Import, export and share', () => {
  const directory = mkdtempSync(join(tmpdir(), 'devnotes-e2e-'));

  /** Forward slashes: `\` is an escape on the wire and a separator on Windows. */
  const bundlePath = join(directory, 'library.devnotes').replaceAll('\\', '/');

  /** Kept from `before`: the seeded space is named from a translation (see below). */
  let homeId = '';

  before(async () => {
    await canvas.open();
    homeId = await homeSpaceId();
    await bridge.createNote(draft({ spaceId: homeId, title: 'Worth exporting', content: 'echo hello' }));
    await reloadCanvas();
  });

  it('writes every note to the bundle', async () => {
    // The sample corpus plus the note seeded above, in the one seeded space.
    const corpus = await bridge.queryNotes(query());
    const written = await bridge.exportNotes(bundlePath);
    expect(written.notes).toBe(corpus.matched);
    expect(written.spaces).toBe((await bridge.listSpaces()).length);

    expect(existsSync(bundlePath)).toBe(true);

    // An archive, not JSON: the attachments travel as entries beside the bundle. What
    // the archive holds is asserted in `tests/transfer.rs`, which can open one — reading
    // a deflated entry from here would mean a zip reader in the harness for one check.
    expect(readFileSync(bundlePath).subarray(0, 4)).toEqual(Buffer.from('PK\x03\x04', 'binary'));
  });

  it('imports nothing when every note is already there', async () => {
    const before = (await bridge.queryNotes(query())).matched;
    const report = await bridge.importNotes(bundlePath);

    // Correct, and only the report says so: every id in the bundle is already there.
    expect(report.notesImported).toBe(0);
    expect(report.notesSkipped).toBe(before);
    expect((await bridge.queryNotes(query())).matched).toBe(before);
  });

  it('brings a note back once it is really gone', async () => {
    const view = await bridge.queryNotes(query({ search: 'Worth exporting' }));
    const id = view.sections[0]?.notes[0]?.id;
    expect(id).toBeDefined();

    await bridge.deleteNote(id!);
    await browser.executeAsync((noteId: string, done: (value: unknown) => void) => {
      const tauri = (window as unknown as Record<string, any>)['__TAURI__'];
      tauri.core
        .invoke('purge_notes', { ids: [noteId] })
        .then(() => done(null))
        .catch(() => done(null));
    }, id!);

    const report = await bridge.importNotes(bundlePath);
    expect(report.notesImported).toBe(1);

    await reloadCanvas();
    await canvas.waitForCard('Worth exporting');
  });

  it('files an imported note into the existing space rather than a second one', async () => {
    const before = await bridge.listSpaces();
    // By identity, never by name: the seeded space is translated and the application
    // opens in the system language, so a spec that spells the name fails on CI.
    expect(before.some((space) => space.id === homeId)).toBe(true);

    // Matching is case-insensitive and by name, so nothing is created.
    const report = await bridge.importNotes(bundlePath);
    expect(report.spacesCreated).toBe(0);

    const after = await bridge.listSpaces();
    expect(after.map((space) => space.name)).toEqual(before.map((space) => space.name));

    const view = await bridge.queryNotes(query({ search: 'Worth exporting' }));
    expect(view.sections[0]?.notes[0]?.spaceId).toBe(homeId);
  });

  /**
   * `language` and `kind` are closed enums on both sides, so a note from a newer DevNotes
   * arrives with that field brought down to the default rather than failing the file.
   */
  it('imports a bundle from a newer version instead of refusing it whole', async () => {
    // Written as a bare `.json`, which is also the shape DevNotes exported before the
    // archive: this doubles as the proof that an old export still imports.
    const exported = await bridge.queryNotes(query({ search: 'Worth exporting' }));
    const source = exported.sections[0]?.notes[0];
    expect(source).toBeDefined();

    const newerPath = join(directory, 'newer.json').replaceAll('\\', '/');
    writeFileSync(
      newerPath,
      JSON.stringify({
        version: 1,
        exportedAt: new Date().toISOString(),
        spaces: await bridge.listSpaces(),
        notes: [
          {
            ...source,
            id: 'written-by-a-newer-devnotes',
            title: 'Ahead of this build',
            language: 'from-the-future',
          },
        ],
      }),
      'utf8',
    );

    const report = await bridge.importNotes(newerPath);
    expect(report.notesImported).toBe(1);
    expect(report.notesDegraded).toBe(1);

    const view = await bridge.queryNotes(query({ search: 'Ahead of this build' }));
    expect(view.sections[0]?.notes[0]?.language).toBe('txt');
  });

  /**
   * The export is the one file the library key does not protect: it is meant to reach
   * another machine, so it carries a key of its own. Read from Node, against the bytes on
   * disk rather than against what the application says about them.
   */
  it('seals an export with a phrase, and will not open it without that phrase', async () => {
    const sealedPath = join(directory, 'sealed.devnotes').replaceAll('\\', '/');

    const written = await bridge.exportNotes(sealedPath, null, 'an export passphrase');
    expect(written.protected).toBe(true);
    expect(await bridge.exportIsProtected(sealedPath)).toBe(true);
    expect(await bridge.exportIsProtected(bundlePath)).toBe(false);
    expect(readFileSync(sealedPath).includes(Buffer.from('Worth exporting'))).toBe(false);

    const refused = await failureOf(bridge.importNotes(sealedPath));
    expect(refused).toContain('passphraseRequired');

    const wrong = await failureOf(bridge.importNotes(sealedPath, 'not the phrase'));
    expect(wrong).toContain('wrongPassphrase');

    const report = await bridge.importNotes(sealedPath, 'an export passphrase');
    expect(report.notesSkipped).toBeGreaterThan(0);
  });
  it('greys out the menu entries that have nothing to act on', async () => {
    await fileMenu.open();
    // The entry stays in the DOM and clickable — it carries `aria-disabled`, not `disabled`.
    expect(await fileMenu.isDisabled('exportSelection')).toBe(true);
    expect(await fileMenu.isDisabled('exportAll')).toBe(false);

    // Neither is clicked: both open the OS file picker, which blocks the application
    // until a human answers it.
    expect(await fileMenu.entry('exportAll').isExisting()).toBe(true);
    await press('Escape');
  });
});
