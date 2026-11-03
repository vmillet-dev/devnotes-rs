import { browser, expect } from '@wdio/globals';

import { canvas } from '../pageobjects/canvas.page.js';
import { board } from '../pageobjects/board.page.js';
import { banners, fileMenu, titlebar } from '../pageobjects/titlebar.page.js';
import { bridge, homeSpaceId, query } from '../support/bridge.js';

/**
 * A database file that does not exist yet: migrations against a real path, the seeding
 * guards, and a front end that boots far enough to render what came back.
 *
 * Every spec file starts on such a profile; this one asserts what the others build on.
 */
describe('First launch', () => {
  /** A scenario that throws on the board would leave every later one reading it. */
  afterEach(async () => {
    await board.show('list');
  });

  // The gate first: no command is answered — not even a read — until it has been passed,
  // and it waits for the samples to land.
  before(canvas.open);

  it('opens on a window wearing the application name', async () => {
    // From `Cargo.toml` through `APP_METADATA`, not `tauri.conf.json`'s lowercase name.
    expect(await titlebar.title()).toBe('DevNotes');
  });

  it('seeds one space and six sample notes', async () => {
    const spaces = await bridge.listSpaces();
    expect(spaces).toHaveLength(1);

    expect(await canvas.cards().length).toBe(6);

    // Recorded by the gate while the guarantee above held; every spec file reads it back.
    expect(await homeSpaceId()).toBe(spaces[0]?.id);
  });

  /**
   * Written in the same transaction as the space and the notes: a seeding that left the
   * folders out would be permanent, because a space exists and both of the front end's
   * guards then read "already seeded".
   */
  /**
   * Nothing here compares a seeded name: the samples are translated, and the locale
   * comes from the system — French on a developer's machine, English on the runners. What
   * is asserted is the shape and the order, which are the same in every language.
   */
  it('seeds its folders too, in an order that does not depend on luck', async () => {
    const folders = await bridge.listFolders(await homeSpaceId());
    expect(folders).toHaveLength(2);

    // A millisecond apart. Sharing one instant left `created_at` tying and the order
    // falling back to a random UUID, so the two zones swapped between installs.
    const at = folders.map((folder) => new Date(folder.createdAt).getTime());
    expect(at[0]).toBeLessThan(at[1]!);

    // Assigned by rotation, so the two zones are told apart on the board at a glance.
    expect(folders[0]?.colour).not.toBe(folders[1]?.colour);
  });

  /**
   * Each sample shows one thing: two Notes (the welcome, one with tasks, a table and code), three
   * snippets (fields, a JSON to open as a graph, code to format) and a todo list.
   */
  it('seeds each kind, the snippets in their languages, and the todo list with a deadline and a priority', async () => {
    const notes = (await bridge.queryNotes(query())).sections.flatMap((section) => section.notes);
    const count = (kind: string) => notes.filter((note) => note.kind === kind).length;

    expect([count('note'), count('snippet'), count('checklist')]).toEqual([2, 3, 1]);
    expect(
      notes
        .filter((note) => note.kind === 'snippet')
        .map((note) => note.language)
        .sort(),
    ).toEqual(['json', 'sh', 'ts']);
    const list = notes.find((note) => note.kind === 'checklist');
    expect(list?.priority).toBe('high');
    expect(list?.lifecycle.kind).toBe('expires');
  });

  /** The chip is the affordance that says a note lives somewhere; five of six wear one. */
  it('arrives arranged, with one note left loose on purpose', async () => {
    const filed = await browser.execute(
      (cardSelector: string, chipSelector: string) =>
        [...document.querySelectorAll(cardSelector)].filter((card) => card.querySelector(chipSelector))
          .length,
      '[data-testid="note-card"]',
      '[data-testid="note-card-folder"]',
    );

    expect(filed).toBe(5);
  });

  /** The screen that explains what a folder is for must not open empty on a fresh install. */
  it('opens a board that already has something on it', async () => {
    const folders = await bridge.listFolders(await homeSpaceId());
    await canvas.open();
    await board.show('board');

    // Against what the store answered, not against a word: the board's order is the
    // store's order, whatever language the names happen to be in.
    expect(await board.zoneNames()).toEqual(folders.map((folder) => folder.name));
    expect(await board.looseTitles()).toHaveLength(1);
  });

  it('gives every card a click surface of its own', async () => {
    const card = await canvas.cardWithTitle((await canvas.titles())[0] ?? '');
    expect(await canvas.cardButton(card).isExisting()).toBe(true);
  });

  it('lights the untriaged filter, because exactly one sample carries a deadline', async () => {
    const everything = await canvas.titles();
    expect(everything).toHaveLength(6);

    await canvas.applyFilter('untriaged');

    // One of the six: a filter that filtered nothing would pass "more than zero".
    const untriaged = await canvas.titles();
    expect(untriaged).toHaveLength(1);
    expect(everything).toContain(untriaged[0]);

    await canvas.applyFilter('all');
    expect(await canvas.titles()).toHaveLength(6);
  });

  it('offers a way out of the application, which nothing here clicks', async () => {
    await fileMenu.open();
    // Presence only. Clicking it quits, and one application serves the whole run.
    expect(await fileMenu.quit().isExisting()).toBe(true);
    await fileMenu.open();
  });

  it('boots without an error banner', async () => {
    // NG0203, a missing capability and a failed migration all land here.
    const banner = banners.error();
    const shown = (await banner.isExisting()) ? await banner.getText() : '';

    expect(shown).toBe('');
  });
});
