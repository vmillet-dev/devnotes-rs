import { browser, expect } from '@wdio/globals';

import { canvas } from '../pageobjects/canvas.page.js';
import { editor } from '../pageobjects/editor.page.js';
import { crumb, folders, selectionBar, spaces } from '../pageobjects/overlays.page.js';
import { eventually, reloadCanvas, testid, waitForCanvas } from '../support/app.js';
import { bridge, draft, homeSpaceId, query } from '../support/bridge.js';

/**
 * `notes.folder_id` carries `ON DELETE SET NULL`, the one thing a folder must never get
 * wrong — and only a real database proves a delete leaves the notes standing.
 */
describe('Folders', () => {
  let homeId = '';

  /**
   * ⚠️ The active space is front-end state, so `browser.refresh()` drops it back to "all
   * spaces" — where the switcher offers no creation, a folder having nowhere to go. Every
   * reload here therefore settles back into the home space.
   */
  async function reloadInHomeSpace(): Promise<void> {
    await reloadCanvas();
    await spaces.open();
    await spaces.option(homeId).click();
    await waitForCanvas();
  }

  before(async () => {
    await canvas.open();
    homeId = await homeSpaceId();

    // Established, never inherited: one application serves the whole run.
    for (const folder of await bridge.listFolders()) {
      await bridge.deleteFolder(folder.id);
    }
    await reloadInHomeSpace();
  });

  after(async () => {
    for (const folder of await bridge.listFolders()) {
      await bridge.deleteFolder(folder.id);
    }
    await spaces.open();
    await spaces.allOption().click();
    await reloadCanvas();
  });

  it('creates a folder from the switcher', async () => {
    await folders.open();
    await folders.create('Migrations');

    const made = await eventually(
      () => bridge.listFolders(homeId),
      (listed) => listed.some((folder) => folder.name === 'Migrations'),
      'the new folder to be listed',
    );
    expect(made.map((folder) => folder.name)).toEqual(['Migrations']);
  });

  /** Assigned rather than chosen, so two made back to back never come out the same. */
  it('gives the next one a different colour without being asked', async () => {
    await folders.open();
    await folders.create('Perf');

    const made = await eventually(
      () => bridge.listFolders(homeId),
      (listed) => listed.some((folder) => folder.name === 'Perf'),
      'the second folder to be listed',
    );
    expect(made.map((folder) => folder.name)).toEqual(['Migrations', 'Perf']);
    expect(made[0]?.colour).not.toBe(made[1]?.colour);
  });

  /** The rail is a tree, so a space folds away with its folders and comes back with them. */
  it('folds a space away without changing what is active', async () => {
    await spaces.collapse(homeId);
    expect(await folders.names()).not.toContain('Perf');

    await spaces.collapse(homeId);
    expect(await folders.names()).toContain('Perf');
  });

  it('refuses a second folder of the same name in one space', async () => {
    const refused = await bridge.createFolder({ spaceId: homeId, name: 'migrations' }).then(
      () => 'it was not refused',
      (error: Error) => error.message,
    );

    expect(refused).toContain('duplicateFolderName');
    expect(await bridge.listFolders(homeId)).toHaveLength(2);
  });

  describe('filing notes from the selection bar', () => {
    let perfId = '';

    before(async () => {
      perfId = (await bridge.listFolders(homeId)).find((folder) => folder.name === 'Perf')!.id;
      await bridge.createNote(draft({ spaceId: homeId, title: 'Locks sur jobs' }));
      await bridge.createNote(draft({ spaceId: homeId, title: 'Cache hit ratio' }));
      await reloadInHomeSpace();
    });

    it('files a whole selection in one gesture', async () => {
      await canvas.check('Locks sur jobs');
      await canvas.check('Cache hit ratio');
      await selectionBar.fileInto(perfId);

      const view = await eventually(
        () => bridge.queryNotes(query({ spaceId: homeId, folderId: perfId })),
        (filed) => filed.matched === 2,
        'both ticked notes to be filed',
      );
      expect(view.matched).toBe(2);
    });

    /** The back end resolves the folder; the card never joins an id against a list. */
    it('says on the card where the note lives', async () => {
      await selectionBar.clear();
      await reloadInHomeSpace();

      const card = await canvas.cardWithTitle('Locks sur jobs');
      expect(await card.$(testid('note-card-folder')).getText()).toContain('Perf');
    });

    /** ⚠️ The absence reads on its own; an "unfiled" chip would soil every loose card. */
    it('shows no chip at all on a note with no folder', async () => {
      await bridge.createNote(draft({ spaceId: homeId, title: 'Hors dossier' }));
      await reloadInHomeSpace();

      const card = await canvas.cardWithTitle('Hors dossier');
      expect(await card.$(testid('note-card-folder')).isExisting()).toBe(false);
    });

    /**
     * Choosing a folder *is* opening it: the canvas narrows and the breadcrumb takes the
     * switcher's place, because from inside a folder there is one place to go.
     */
    it('narrows the canvas to one folder, and back out again', async () => {
      await folders.open();
      await folders.option(perfId).click();
      await canvas.waitForNoCard('Hors dossier');

      expect((await canvas.titles()).sort()).toEqual(['Cache hit ratio', 'Locks sur jobs']);
      expect(await crumb.name()).toBe('Perf');

      await crumb.back();
      await canvas.waitForCard('Hors dossier');
      expect(await crumb.isShowing()).toBe(false);
    });

    /** The same control both ways: taking a note out is a filing with no folder. */
    it('takes a selection back out of its folder', async () => {
      await canvas.check('Cache hit ratio');
      await selectionBar.fileInto(null);

      const view = await eventually(
        () => bridge.queryNotes(query({ spaceId: homeId, folderId: perfId })),
        (left) => left.matched === 1,
        'the unfiled note to leave the folder',
      );
      await selectionBar.clear();
      expect(view.matched).toBe(1);
    });

    /** ⚠️ The chip would otherwise name a folder the space switcher can never reach. */
    it('unfiles a note carried off to another space', async () => {
      const elsewhere = await bridge.createSpace({ name: 'Ailleurs' });
      const id = await canvas.noteIdWithTitle('Locks sur jobs');

      await bridge.moveNotes([id], elsewhere.id);

      const view = await bridge.queryNotes(query({ spaceId: elsewhere.id }));
      expect(view.sections[0]?.notes[0]?.folderId ?? null).toBeNull();

      await bridge.deleteSpace(elsewhere.id, homeId);
      await reloadInHomeSpace();
    });
  });

  it('renames a folder and recolours it without touching its id', async () => {
    const before = (await bridge.listFolders(homeId)).find((folder) => folder.name === 'Migrations')!;

    await folders.open();
    await folders.rename(before.id, 'Schéma');
    await eventually(
      async () => (await bridge.listFolders(homeId)).find((folder) => folder.id === before.id),
      (folder) => folder?.name === 'Schéma',
      'the rename to land before the colour is changed',
    );

    await folders.open();
    await folders.recolour(before.id, 'red');

    const after = await eventually(
      async () => (await bridge.listFolders(homeId)).find((folder) => folder.id === before.id),
      (folder) => folder?.colour === 'red',
      'the new colour to be stored',
    );
    expect(after?.name).toBe('Schéma');
    expect(after?.colour).toBe('red');
  });

  /**
   * The whole point of `ON DELETE SET NULL`: a folder is a label on a region, never a
   * container that takes its contents with it. A cascade here would be silent data loss.
   *
   * ⚠️ Seeds its own folder and note rather than reusing the ones above: Mocha runs a
   * nested suite after its siblings, so the filing block below has not run yet.
   */
  it('leaves every note standing when the folder goes', async () => {
    const doomed = await bridge.createFolder({ spaceId: homeId, name: 'Jetable' });
    const note = await bridge.createNote(draft({ spaceId: homeId, title: 'Survivante' }));
    await bridge.fileNotes([note.id], doomed.id);
    await reloadInHomeSpace();

    expect((await bridge.queryNotes(query({ spaceId: homeId, folderId: doomed.id }))).matched).toBe(1);
    const corpus = (await bridge.queryNotes(query({ spaceId: homeId }))).matched;

    await folders.open();
    await folders.remove(doomed.id);

    await eventually(
      () => bridge.listFolders(homeId),
      (listed) => !listed.some((folder) => folder.name === 'Jetable'),
      'the deleted folder to be gone',
    );
    expect((await bridge.queryNotes(query({ spaceId: homeId }))).matched).toBe(corpus);

    // Standing, and now loose — the chip is what a `SET NULL` takes away.
    const view = await bridge.queryNotes(query({ search: 'Survivante' }));
    expect(view.matched).toBe(1);
    expect(view.sections[0]?.notes[0]?.folderId ?? null).toBeNull();
  });

  /** Its folders go with it through the cascade, and its notes come out of the refuge loose. */
  it('drops the folders of a deleted space and unfiles the notes it hands over', async () => {
    const doomed = await bridge.createSpace({ name: 'Éphémère' });
    const folder = await bridge.createFolder({ spaceId: doomed.id, name: 'Temporaire' });
    const note = await bridge.createNote(draft({ spaceId: doomed.id, title: 'Rescapée' }));
    await bridge.fileNotes([note.id], folder.id);

    await bridge.deleteSpace(doomed.id, homeId);

    expect(await bridge.listFolders(doomed.id)).toHaveLength(0);
    const view = await bridge.queryNotes(query({ search: 'Rescapée' }));
    expect(view.matched).toBe(1);
    expect(view.sections[0]?.notes[0]?.folderId ?? null).toBeNull();
    expect(view.sections[0]?.notes[0]?.spaceId).toBe(homeId);

    await reloadInHomeSpace();
  });

  /**
   * A note's properties used to be spread over four surfaces, and none of them was
   * complete: filing one from the date view took three clicks through the selection bar,
   * and pinning with the mouse took a full-screen modal for a boolean with its own chip in
   * the header.
   */
  describe('the two surfaces about one note', () => {
    let folderId = '';
    const title = 'Sauvegarde nocturne';

    before(async () => {
      folderId = (await bridge.createFolder({ spaceId: homeId, name: 'Exploitation' })).id;
      await bridge.createNote(draft({ spaceId: homeId, title }));
      await reloadInHomeSpace();
      await canvas.waitForCard(title);
    });

    it('offers everything a note can be told to do, from the card', async () => {
      const entries = await canvas.cardMenuEntries(title);
      await browser.keys('Escape');

      expect(entries).toEqual(
        expect.arrayContaining([
          'note-card-open',
          'note-card-pin',
          'note-card-copy',
          'note-card-file',
          'note-card-move',
          'note-card-delete',
        ]),
      );
    });

    /** ⚠️ One gesture, where the selection bar took three clicks and a fourth to clear. */
    it('files a note into a folder from the card, in one gesture', async () => {
      await canvas.fileNote(title, folderId);

      const view = await eventually(
        () => bridge.queryNotes(query({ search: title })),
        (found) => found.sections[0]?.notes[0]?.folderId === folderId,
        'the filing to reach the database',
      );
      expect(view.sections[0]?.notes[0]?.folderId).toBe(folderId);
    });

    it('takes it back out from the same menu', async () => {
      await canvas.fileNote(title, null);

      const view = await eventually(
        () => bridge.queryNotes(query({ search: title })),
        (found) => (found.sections[0]?.notes[0]?.folderId ?? null) === null,
        'the unfiling to reach the database',
      );
      expect(view.sections[0]?.notes[0]?.folderId ?? null).toBeNull();
    });

    /** ⚠️ Three clicks and a full-screen modal, for a boolean, until now. */
    it('pins with the mouse without opening anything', async () => {
      await canvas.pinFromCardMenu(title);

      const view = await eventually(
        () => bridge.queryNotes(query({ search: title })),
        (found) => found.sections[0]?.notes[0]?.pinned === true,
        'the pin to reach the database',
      );
      expect(view.sections[0]?.notes[0]?.pinned).toBe(true);
      await canvas.pinFromCardMenu(title);
    });

    /**
     * ⚠️ The editor was the one surface about a single note that could not move it. A
     * folder belongs to one space, so the two controls are shown together: moving the
     * space clears the folder, and a folder control alone would lie about it.
     */
    it('files from the editor too, which could not move a note at all', async () => {
      await canvas.openNote(title);
      const before = await editor.placementLabel('folder');

      await editor.place('folder', folderId);

      const view = await eventually(
        () => bridge.queryNotes(query({ search: title })),
        (found) => found.sections[0]?.notes[0]?.folderId === folderId,
        'the editor filing to reach the database',
      );
      expect(view.sections[0]?.notes[0]?.folderId).toBe(folderId);
      // ⚠️ The name, not a translated "no folder": the suite switches language partway.
      expect(before).not.toContain('Exploitation');
      expect(await editor.placementLabel('folder')).toContain('Exploitation');
      await editor.close();
    });

    /**
     * ⚠️ This menu lives inside a dialog, unlike every other one in the application: the
     * trigger lets Escape bubble on purpose, and the next listener up is the editor's own.
     * One Escape closed the note along with the menu.
     */
    it('folds the placement menu on Escape without closing the note', async () => {
      await canvas.openNote(title);
      await $(testid('editor-placement-folder')).click();
      await $(testid('editor-placement-option')).waitForExist({ timeout: 5_000 });

      await browser.keys('Escape');

      await $(testid('editor-placement-option')).waitForExist({ reverse: true, timeout: 5_000 });
      expect(await editor.isOpen()).toBe(true);
      await editor.close();
    });

    it('moves the note to another space from the editor, which clears its folder', async () => {
      const elsewhere = await bridge.createSpace({ name: 'Ailleurs' });
      // ⚠️ The bridge writes straight to the database, so the front end has never heard of
      // that space: without this the menu it offers has no such entry to click.
      await reloadInHomeSpace();
      await canvas.openNote(title);

      await editor.place('space', elsewhere.id);

      const view = await eventually(
        () => bridge.queryNotes(query({ search: title })),
        (found) => found.sections[0]?.notes[0]?.spaceId === elsewhere.id,
        'the move to reach the database',
      );
      expect(view.sections[0]?.notes[0]?.folderId ?? null).toBeNull();

      await editor.close();
      await bridge.deleteSpace(elsewhere.id, homeId);
      await reloadInHomeSpace();
    });
  });
});
