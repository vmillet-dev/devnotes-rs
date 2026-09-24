import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { StatusNotifier } from '@core/services/notifications/status.service';
import { FakeClipboard } from '@testing/fake-clipboard';
import { FakeFileDialog } from '@testing/fake-file-dialog';
import { FakeTransferRepository } from '@testing/fake-transfer-repository';
import { provideAppTesting } from '@testing/testing.providers';
import { PassphraseAnswer, PassphrasePromptStore } from './passphrase-prompt.store';
import { TransferStore } from './transfer.store';

const NOW = new Date('2026-08-27T09:00:00Z');

/** Declining the protection: what most of these scenarios are not about. */
const IN_THE_CLEAR: PassphraseAnswer = { kind: 'none' };

interface Harness {
  readonly store: TransferStore;
  readonly prompt: PassphrasePromptStore;
  readonly repository: FakeTransferRepository;
  readonly dialog: FakeFileDialog;
  readonly clipboard: FakeClipboard;
  readonly status: StatusNotifier;
  readonly notifier: ErrorNotifier;
}

function createStore(): Harness {
  TestBed.resetTestingModule();
  const repository = new FakeTransferRepository();
  const dialog = new FakeFileDialog();
  const clipboard = new FakeClipboard();
  TestBed.configureTestingModule({
    providers: [provideAppTesting({ transferRepository: repository, fileDialog: dialog, clipboard })],
  });

  return {
    store: TestBed.inject(TransferStore),
    prompt: TestBed.inject(PassphrasePromptStore),
    repository,
    dialog,
    clipboard,
    status: TestBed.inject(StatusNotifier),
    notifier: TestBed.inject(ErrorNotifier),
  };
}

/** The prompt is a promise the store is waiting on; nothing advances until it is given one. */
async function asking(harness: Harness): Promise<void> {
  for (let turn = 0; turn < 50; turn++) {
    if (harness.prompt.request() !== null && !harness.prompt.working()) return;
    await Promise.resolve();
  }
}

async function answer(harness: Harness, ...answers: PassphraseAnswer[]): Promise<void> {
  for (const given of answers) {
    await asking(harness);

    expect(harness.prompt.request()).not.toBeNull();
    harness.prompt.answer(given);
  }
}

async function exportEverything(
  harness: Harness,
  spaceId: string | null = null,
  given: PassphraseAnswer = IN_THE_CLEAR,
  now: Date = NOW,
): Promise<void> {
  const done = harness.store.export(spaceId, now);
  await answer(harness, given);
  await done;
}

describe('TransferStore', () => {
  let harness: Harness;

  beforeEach(() => {
    harness = createStore();
  });

  describe('import', () => {
    it('does nothing when the dialog is cancelled', async () => {
      harness.dialog.openPath = null;

      expect(await harness.store.import()).toBe(false);
      expect(harness.repository.importedFrom).toBeNull();
      expect(harness.status.status()).toBeNull();
    });

    it('reports what came in, naming the file it read', async () => {
      harness.dialog.openPath = 'C:/notes/devnotes-2026-08-27.devnotes';

      expect(await harness.store.import()).toBe(true);
      expect(harness.status.status()).toEqual({
        key: 'file.imported',
        params: {
          notes: '2',
          skipped: '0',
          degraded: '0',
          attachments: '0',
          missing: '0',
          folders: '0',
          path: 'devnotes-2026-08-27.devnotes',
        },
      });
    });

    /** A library received arranged is worth saying so: the folders are half of what came in. */
    it('says how many folders a library arrived with', async () => {
      harness.dialog.openPath = 'C:/notes/devnotes.devnotes';
      harness.repository.importReport = {
        ...harness.repository.importReport,
        notesImported: 6,
        foldersCreated: 3,
      };

      expect(await harness.store.import()).toBe(true);
      expect(harness.status.status()?.key).toBe('file.importedIntoFolders');
      expect(harness.status.status()?.params?.['folders']).toBe('3');
    });

    /** Every library operation reports, including when it changed nothing. */
    it('counts a library that brought folders but no new note as a change', async () => {
      harness.dialog.openPath = 'C:/notes/devnotes.devnotes';
      harness.repository.importReport = {
        ...harness.repository.importReport,
        notesImported: 0,
        notesSkipped: 4,
        foldersCreated: 2,
      };

      expect(await harness.store.import()).toBe(true);
    });

    // A bundle from a newer DevNotes imports rather than failing whole, and the note it
    // brought down to a language this build knows is the only trace of it.
    it('says when a note came from a newer version', async () => {
      harness.dialog.openPath = 'C:/in.json';
      harness.repository.importReport = {
        foldersCreated: 0,
        spacesCreated: 0,
        notesImported: 5,
        notesSkipped: 0,
        notesDegraded: 1,
        attachmentsImported: 0,
        attachmentsMissing: 0,
      };

      expect(await harness.store.import()).toBe(true);
      expect(harness.status.status()?.key).toBe('file.importedFromNewerVersion');
      expect(harness.status.status()?.params).toMatchObject({ notes: '5', degraded: '1' });
    });

    it('counts the attachments that came back with the notes', async () => {
      harness.dialog.openPath = 'C:/in.devnotes';
      harness.repository.importReport = {
        foldersCreated: 0,
        spacesCreated: 0,
        notesImported: 2,
        notesSkipped: 0,
        notesDegraded: 0,
        attachmentsImported: 3,
        attachmentsMissing: 0,
      };

      expect(await harness.store.import()).toBe(true);
      expect(harness.status.status()?.key).toBe('file.importedWithAttachments');
      expect(harness.status.status()?.params).toMatchObject({ attachments: '3' });
    });

    /** The note arrives with a thumbnail that will never load, and this is the only
     *  thing that says why. It outranks the degraded notice on purpose: a missing file is
     *  a defect, a degraded field is a shrug. */
    it('says when the archive named an attachment it did not carry', async () => {
      harness.dialog.openPath = 'C:/in.devnotes';
      harness.repository.importReport = {
        foldersCreated: 0,
        spacesCreated: 0,
        notesImported: 2,
        notesSkipped: 0,
        notesDegraded: 1,
        attachmentsImported: 0,
        attachmentsMissing: 2,
      };

      expect(await harness.store.import()).toBe(true);
      expect(harness.status.status()?.key).toBe('file.importedWithoutSomeAttachments');
      expect(harness.status.status()?.params).toMatchObject({ missing: '2' });
    });

    it('says so plainly when everything was already there', async () => {
      harness.dialog.openPath = 'C:/in.json';
      harness.repository.importReport = {
        foldersCreated: 0,
        spacesCreated: 0,
        notesImported: 0,
        notesSkipped: 4,
        notesDegraded: 0,
        attachmentsImported: 0,
        attachmentsMissing: 0,
      };

      expect(await harness.store.import()).toBe(false);
      expect(harness.status.status()?.key).toBe('file.importedNothing');
      expect(harness.status.status()?.params).toMatchObject({ skipped: '4' });
    });

    it('surfaces a failure and leaves no success message behind', async () => {
      harness.dialog.openPath = 'C:/in.json';
      harness.repository.failNext = new Error('boom');

      expect(await harness.store.import()).toBe(false);
      expect(harness.notifier.notice()?.ref.key).toBe('errors.importFailed');
      expect(harness.status.status()).toBeNull();
    });

    it('asks nothing of a file that is not protected', async () => {
      harness.dialog.openPath = 'C:/in.devnotes';

      expect(await harness.store.import()).toBe(true);
      expect(harness.prompt.request()).toBeNull();
      expect(harness.repository.importedWith).toBeNull();
    });

    it('asks for the phrase a protected file was sealed with, and hands it over', async () => {
      harness.dialog.openPath = 'C:/in.devnotes';
      harness.repository.fileIsProtected = true;
      harness.repository.expectedPassphrase = 'the shared phrase';

      const done = harness.store.import();
      await answer(harness, { kind: 'phrase', value: 'the shared phrase' });

      expect(await done).toBe(true);
      expect(harness.repository.importedWith).toBe('the shared phrase');
    });

    /** A typo must not cost the import: the phrase is asked for again, and the refusal
     *  is said beside the field rather than in the error banner. */
    it('asks again when the phrase is refused', async () => {
      harness.dialog.openPath = 'C:/in.devnotes';
      harness.repository.fileIsProtected = true;
      harness.repository.expectedPassphrase = 'the shared phrase';

      const done = harness.store.import();
      await answer(harness, { kind: 'phrase', value: 'a typo' });
      await answer(harness, { kind: 'phrase', value: 'the shared phrase' });

      expect(await done).toBe(true);
      expect(harness.notifier.notice()).toBeNull();
      expect(harness.status.status()?.key).toBe('file.imported');
    });

    it('says the phrase was refused the second time it asks', async () => {
      harness.dialog.openPath = 'C:/in.devnotes';
      harness.repository.fileIsProtected = true;
      harness.repository.expectedPassphrase = 'the shared phrase';

      const done = harness.store.import();
      await answer(harness, { kind: 'phrase', value: 'a typo' });
      await asking(harness);

      expect(harness.prompt.request()).toMatchObject({ purpose: 'unlock', refused: true });

      harness.prompt.answer({ kind: 'cancelled' });
      await done;
    });

    it('imports nothing and reports no failure when the prompt is given up on', async () => {
      harness.dialog.openPath = 'C:/in.devnotes';
      harness.repository.fileIsProtected = true;

      const done = harness.store.import();
      await answer(harness, { kind: 'cancelled' });

      expect(await done).toBe(false);
      expect(harness.notifier.notice()).toBeNull();
      expect(harness.status.status()).toBeNull();
      expect(harness.repository.importedWith).toBeNull();
    });
  });

  describe('export', () => {
    /**
     * Down to the minute: two exports on the same day were offered the same name, and
     * replacing the first was one Enter away. The expectation is derived through
     * `sv-SE`, which formats local time ISO-style — a literal would pin the runner's
     * timezone, and the point is that the name is **not** the UTC day.
     */
    it('proposes a name stamped with the local day and minute', async () => {
      harness.dialog.savePath = 'C:/out.json';
      const local = NOW.toLocaleString('sv-SE');
      const expected = `devnotes-${local.slice(0, 10)}-${local.slice(11, 13)}${local.slice(14, 16)}.devnotes`;

      await exportEverything(harness);

      expect(harness.dialog.saveCalls[0].defaultPath).toBe(expected);
    });

    it('offers a second name a minute later, rather than the first one again', async () => {
      harness.dialog.savePath = 'C:/out.json';
      await exportEverything(harness);

      await exportEverything(harness, null, IN_THE_CLEAR, new Date(NOW.getTime() + 60_000));

      const [first, second] = harness.dialog.saveCalls;
      expect(second?.defaultPath).not.toBe(first?.defaultPath);
    });

    it('passes the active space through, or null for everything', async () => {
      harness.dialog.savePath = 'C:/out.json';

      await exportEverything(harness, 'space-1');
      expect(harness.repository.exportedTo).toEqual({
        path: 'C:/out.json',
        spaceId: 'space-1',
        passphrase: null,
      });

      await exportEverything(harness);
      expect(harness.repository.exportedTo?.spaceId).toBeNull();
    });

    it('writes nothing when no destination is chosen', async () => {
      harness.dialog.savePath = null;

      await harness.store.export(null, NOW);

      expect(harness.prompt.request()).toBeNull();
      expect(harness.repository.exportedTo).toBeNull();
      expect(harness.status.status()).toBeNull();
    });

    it('says how many notes went out, and where', async () => {
      harness.dialog.savePath = 'C:/backups/devnotes.devnotes';

      await exportEverything(harness);

      expect(harness.status.status()).toEqual({
        key: 'file.exported',
        params: { notes: '3', attachments: '0', path: 'devnotes.devnotes' },
      });
    });

    /** The attachments now travel, and a report that stayed silent about them would let a
     *  user believe an export of screenshots carried none. */
    it('counts the attachments that travelled with the notes', async () => {
      harness.dialog.savePath = 'C:/backups/devnotes.devnotes';
      harness.repository.exportReport = { notes: 3, spaces: 1, folders: 0, attachments: 2, protected: false };

      await exportEverything(harness);

      expect(harness.status.status()).toEqual({
        key: 'file.exportedWithAttachments',
        params: { notes: '3', attachments: '2', path: 'devnotes.devnotes' },
      });
    });

    it('does not pretend to have exported an empty library', async () => {
      harness.dialog.savePath = 'C:/out.json';
      harness.repository.exportReport = { notes: 0, spaces: 0, folders: 0, attachments: 0, protected: false };

      await exportEverything(harness);

      expect(harness.status.status()?.key).toBe('file.emptyLibrary');
    });

    it('restricts the file to the selection when asked', async () => {
      harness.dialog.savePath = 'C:/out.json';

      const done = harness.store.exportSelection(['note-1', 'note-2'], NOW);
      await answer(harness, IN_THE_CLEAR);
      await done;

      expect(harness.repository.exportedIds).toEqual(['note-1', 'note-2']);
      expect(harness.status.status()?.params).toMatchObject({ notes: '2' });
    });

    it('asks for a selection rather than exporting everything', async () => {
      await harness.store.exportSelection([], NOW);

      expect(harness.notifier.notice()?.ref.key).toBe('file.needsSelection');
      expect(harness.dialog.saveCalls).toHaveLength(0);
    });

    /** The file is the one thing here most likely to leave the machine: the phrase goes
     *  through to the command, and the report says the file was sealed with it. */
    it('seals the file with the phrase that was given', async () => {
      harness.dialog.savePath = 'C:/backups/devnotes.devnotes';

      await exportEverything(harness, null, { kind: 'phrase', value: 'a shared phrase' });

      expect(harness.repository.exportedTo?.passphrase).toBe('a shared phrase');
      expect(harness.status.status()).toEqual({
        key: 'file.exportedProtected',
        params: { notes: '3', attachments: '0', path: 'devnotes.devnotes' },
      });
    });

    it('asks after the destination is known, naming the file it is about to write', async () => {
      harness.dialog.savePath = 'C:/backups/devnotes.devnotes';

      const done = harness.store.export(null, NOW);
      await asking(harness);

      expect(harness.prompt.request()).toEqual({
        purpose: 'protect',
        fileName: 'devnotes.devnotes',
        refused: false,
      });

      harness.prompt.answer(IN_THE_CLEAR);
      await done;
    });

    it('writes nothing when the prompt is cancelled', async () => {
      harness.dialog.savePath = 'C:/out.devnotes';

      const done = harness.store.export(null, NOW);
      await answer(harness, { kind: 'cancelled' });
      await done;

      expect(harness.repository.exportedTo).toBeNull();
      expect(harness.status.status()).toBeNull();
    });
  });

  /** A dialog that vanished and came back on a typo would read as a fault, so the
   *  prompt stays up while the phrase is being derived from — and refuses a second answer. */
  it('keeps the prompt on screen while the phrase is being used', async () => {
    harness.dialog.openPath = 'C:/in.devnotes';
    harness.repository.fileIsProtected = true;
    harness.repository.expectedPassphrase = 'the shared phrase';

    const done = harness.store.import();
    await answer(harness, { kind: 'phrase', value: 'a typo' });

    expect(harness.prompt.request()).not.toBeNull();
    expect(harness.prompt.working()).toBe(true);

    harness.prompt.answer({ kind: 'cancelled' });
    expect(harness.prompt.working()).toBe(true);

    await answer(harness, { kind: 'cancelled' });
    expect(await done).toBe(false);
    expect(harness.prompt.request()).toBeNull();
  });

  describe('copy as Markdown', () => {
    it('asks for a selection rather than copying everything', async () => {
      await harness.store.copyAsMarkdown([]);

      expect(harness.notifier.notice()?.ref.key).toBe('file.needsSelection');
      expect(harness.repository.sharedIds).toBeNull();
    });

    it('puts the markdown in the clipboard and goes no further', async () => {
      await harness.store.copyAsMarkdown(['note-1', 'note-2']);

      expect(harness.repository.sharedIds).toEqual(['note-1', 'note-2']);
      expect(harness.clipboard.content).toBe(harness.repository.markdown);
      expect(harness.status.status()).toEqual({
        key: 'file.copied',
        params: { notes: '2' },
      });
    });

    it('says so when the clipboard refuses', async () => {
      harness.clipboard.failNext = new Error('no clipboard');

      await harness.store.copyAsMarkdown(['note-1']);

      expect(harness.notifier.notice()?.ref.key).toBe('errors.copyFailed');
      expect(harness.status.status()).toBeNull();
    });
  });

  it('is idle again once an operation ends', async () => {
    harness.dialog.openPath = 'C:/in.json';

    await harness.store.import();

    expect(harness.store.isBusy()).toBe(false);
  });
});
