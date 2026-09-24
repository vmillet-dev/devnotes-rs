import { browser, expect } from '@wdio/globals';
import { mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { canvas } from '../pageobjects/canvas.page.js';
import { editor } from '../pageobjects/editor.page.js';
import { eventually, reloadCanvas } from '../support/app.js';
import { bridge, draft, homeSpaceId, query } from '../support/bridge.js';

/**
 * The bytes are not in the database: the row holds a record and the file lives under
 * `app_data_dir()/attachments/`. The picker is not driven (see `support/app.ts`);
 * `attach_file` takes the path it would have returned.
 */
describe('Attachments', () => {
  const title = 'Note with a file';
  const directory = mkdtempSync(join(tmpdir(), 'devnotes-e2e-'));
  const filePath = join(directory, 'runbook.txt');
  let noteId = '';

  before(async () => {
    writeFileSync(filePath, 'step one\nstep two\n');
    await canvas.open();
    const spaceId = await homeSpaceId();
    noteId = (await bridge.createNote(draft({ spaceId, title }))).id;
    await reloadCanvas();
    await canvas.waitForCard(title);
  });

  /** Forward slashes: `\` is an escape on the wire, a separator on Windows. */
  async function attach(path: string) {
    return browser.executeAsync(
      (id: string, file: string, done: (value: unknown) => void) => {
        const tauri = (window as unknown as Record<string, any>)['__TAURI__'];
        tauri.core
          .invoke('attach_file', { noteId: id, path: file })
          .then((value: unknown) => done({ ok: value }))
          .catch((error: unknown) => done({ err: String(error) }));
      },
      noteId,
      path.replaceAll('\\', '/'),
    ) as Promise<{ ok?: { id: string; fileName: string }; err?: string }>;
  }

  it('says the strip is empty before anything is attached', async () => {
    await canvas.openNote(title);
    expect(await editor.attachmentEmpty().isExisting()).toBe(true);
    expect(await editor.attachments().length).toBe(0);

    // Presence only: clicking it raises the OS file picker, which blocks the whole
    // application until a human answers.
    expect(await editor.attachmentAdd().isExisting()).toBe(true);
    await editor.close();
  });

  it('records the file and keeps its name', async () => {
    const outcome = await attach(filePath);
    expect(outcome.err).toBeUndefined();
    expect(outcome.ok?.fileName).toBe('runbook.txt');

    const listed = await bridge.listAttachments(noteId);
    expect(listed.map((item) => item.fileName)).toEqual(['runbook.txt']);
  });

  it('counts on the card without opening the note', async () => {
    await reloadCanvas();
    const card = await canvas.cardWithTitle(title);
    expect(await card.$('[data-testid="note-card-clip"]').isExisting()).toBe(true);

    const view = await bridge.queryNotes(query({ search: title }));
    expect(view.sections[0]?.notes[0]?.attachmentCount).toBe(1);
  });

  it('lists it in the editor strip, and still does after a reopen', async () => {
    await canvas.openNote(title);
    expect(await editor.attachmentNames()).toEqual(['runbook.txt']);
    expect(await editor.attachmentEmpty().isExisting()).toBe(false);

    await editor.close();
    await canvas.openNote(title);
    expect(await editor.attachmentNames()).toEqual(['runbook.txt']);
  });

  it('offers to hand the file to the desktop, which nothing here clicks', async () => {
    // `open_attachment` asks the OS to launch the default application for the file.
    const open = editor.attachmentOpen('runbook.txt');
    expect(await open.isExisting()).toBe(true);
    expect(await open.getAttribute('aria-label')).toContain('runbook.txt');
    await editor.close();
  });

  it('gives two files of the same name two records', async () => {
    // `model::stored_name` derives the stored name from the record id: two `runbook.txt`
    // must not overwrite each other on disk.
    const second = mkdtempSync(join(tmpdir(), 'devnotes-e2e-'));
    const twin = join(second, 'runbook.txt');
    writeFileSync(twin, 'a different runbook\n');

    const outcome = await attach(twin);
    expect(outcome.err).toBeUndefined();

    const listed = await bridge.listAttachments(noteId);
    expect(listed).toHaveLength(2);
    expect(new Set(listed.map((item) => item.id)).size).toBe(2);
  });

  it('removes one on the second click, and only then', async () => {
    await canvas.openNote(title);

    // By its id, not by position: this note carries two attachments of the same name, so
    // "the first remove button" is not a stable way to click the same one twice.
    const [doomed] = await bridge.listAttachments(noteId);
    const remove = browser.$(`[data-testid="attachment-remove"][data-attachment-id="${doomed?.id}"]`);

    await remove.click();
    // A duration, deliberately: this asserts that nothing happened, and nothing
    // happening is not a condition anything can wait on.
    await browser.pause(400);
    expect(await bridge.listAttachments(noteId)).toHaveLength(2);

    await remove.click();
    const left = await eventually(
      () => bridge.listAttachments(noteId),
      (list) => list.length === 1,
      'the attachment was never removed',
    );

    expect(left.map((item) => item.id)).not.toContain(doomed?.id);
    await editor.close();
  });
});
