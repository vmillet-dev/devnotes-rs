import { expect } from '@wdio/globals';
import { mkdtempSync, readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { canvas } from '../pageobjects/canvas.page.js';
import { reloadCanvas } from '../support/app.js';
import { bridge, draft, homeSpaceId, query } from '../support/bridge.js';

/**
 * The last of the five: a library exported from one machine and imported on another comes
 * back arranged.
 *
 * There is one process and one library for the whole run, so "the other machine" is
 * this one with the notes cleared out — an import that met their ids would skip them, as
 * it is meant to. `tests/transfer.rs` covers the genuinely fresh database.
 */
describe('Folders travelling with a library', () => {
  const directory = mkdtempSync(join(tmpdir(), 'devnotes-folders-'));

  /** Forward slashes: `\` is an escape on the wire and a separator on Windows. */
  const bundlePath = join(directory, 'arranged.devnotes').replaceAll('\\', '/');

  let homeId = '';
  let spaceId = '';
  let noteIds: string[] = [];

  async function folderNames(): Promise<string[]> {
    return (await bridge.listFolders(spaceId)).map((folder) => folder.name).sort();
  }

  async function folderOf(title: string): Promise<string | null> {
    const view = await bridge.queryNotes(query({ spaceId, search: title }));
    const note = view.sections.flatMap((section) => section.notes).find((each) => each.title === title);
    if (!note?.folderId) return null;

    const folders = await bridge.listFolders(spaceId);
    return folders.find((folder) => folder.id === note.folderId)?.name ?? null;
  }

  before(async () => {
    await canvas.open();
    homeId = await homeSpaceId();
    spaceId = (await bridge.createSpace({ name: 'Voyage' })).id;

    const perfId = (await bridge.createFolder({ spaceId, name: 'Perf' })).id;
    const migrationsId = (await bridge.createFolder({ spaceId, name: 'Migrations' })).id;
    // Cited by nobody, so it must not travel — and must survive the import untouched.
    await bridge.createFolder({ spaceId, name: 'Jamais citée' });

    const explain = await bridge.createNote(draft({ spaceId, title: 'EXPLAIN lent' }));
    const backfill = await bridge.createNote(draft({ spaceId, title: 'Backfill UUID' }));
    const dump = await bridge.createNote(draft({ spaceId, title: 'Dump nocturne' }));
    noteIds = [explain.id, backfill.id, dump.id];

    await bridge.fileNotes([explain.id], perfId);
    await bridge.fileNotes([backfill.id], migrationsId);

    await reloadCanvas();
  });

  after(async () => {
    for (const space of await bridge.listSpaces()) {
      if (space.id !== homeId) await bridge.deleteSpace(space.id, homeId);
    }
    await reloadCanvas();
  });

  it('writes the folders the notes actually cite, and no others', async () => {
    const written = await bridge.exportNotes(bundlePath, spaceId);

    expect(written.notes).toBe(3);
    expect(written.folders).toBe(2);
  });

  /**
   * A board received from elsewhere must not land on top of the one you arranged.
   * Keeping the geometry off the `Folder` model is what makes that free rather than a
   * filter to maintain — and this reads the file on disk to prove it.
   */
  it('writes no coordinate into the file at all', () => {
    const written = readFileSync(bundlePath, 'utf8');

    for (const spelled of ['"x"', '"y"', '"w"', '"h"', '"frame"', '"position"']) {
      expect(written).not.toContain(spelled);
    }
  });

  it('puts every note back in the right folder on the other side', async () => {
    // "The other machine": the notes gone for good, and the two cited folders with them.
    await bridge.deleteNotes(noteIds);
    await bridge.purgeNotes(noteIds);
    for (const folder of await bridge.listFolders(spaceId)) {
      if (folder.name !== 'Jamais citée') await bridge.deleteFolder(folder.id);
    }
    expect(await folderNames()).toEqual(['Jamais citée']);

    const report = await bridge.importNotes(bundlePath);

    expect(report.spacesCreated).toBe(0);
    expect(report.foldersCreated).toBe(2);
    expect(report.notesImported).toBe(3);

    expect(await folderNames()).toEqual(['Jamais citée', 'Migrations', 'Perf']);
    expect(await folderOf('EXPLAIN lent')).toBe('Perf');
    expect(await folderOf('Backfill UUID')).toBe('Migrations');
    expect(await folderOf('Dump nocturne')).toBeNull();
  });

  /** The board lays a received folder out itself, on its first read, like any other. */
  it('leaves the receiving board to place what arrived', async () => {
    const view = await bridge.boardView({
      spaceId,
      search: '',
      filter: 'all',
      tags: [],
      languages: [],
      now: new Date().toISOString(),
    });

    expect(view.zones).toHaveLength(3);
    expect(view.zones.every((zone) => zone.frame.width > 0)).toBe(true);
    // Its own layout, laid out here: nothing about it came out of the file.
    expect(new Set(view.zones.map((zone) => zone.frame.x)).size).toBeGreaterThan(1);
  });

  /** Merge, never replace — the rule spaces already follow. */
  it('adds nothing the second time the same file is read', async () => {
    const report = await bridge.importNotes(bundlePath);

    expect(report.foldersCreated).toBe(0);
    expect(report.notesImported).toBe(0);
    expect(report.notesSkipped).toBe(3);
    expect(await folderNames()).toEqual(['Jamais citée', 'Migrations', 'Perf']);
  });
});
