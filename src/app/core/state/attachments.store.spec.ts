import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { StatusNotifier } from '@core/services/notifications/status.service';
import { Attachment } from '@core/model/note.model';
import { FakeAttachmentsRepository } from '@testing/fake-attachments-repository';
import { FakeFileDialog } from '@testing/fake-file-dialog';
import { provideAppTesting } from '@testing/testing.providers';
import { AttachmentsStore } from './attachments.store';

function attachment(overrides: Partial<Attachment> = {}): Attachment {
  return {
    id: 'attachment-1',
    noteId: 'note-1',
    fileName: 'capture.png',
    mimeType: 'image/png',
    byteSize: 2048,
    createdAt: new Date('2026-08-27T09:00:00Z'),
    ...overrides,
  };
}

interface Harness {
  readonly store: AttachmentsStore;
  readonly repository: FakeAttachmentsRepository;
  readonly dialog: FakeFileDialog;
  readonly notifier: ErrorNotifier;
  readonly status: StatusNotifier;
}

function createStore(attachments: readonly Attachment[] = []): Harness {
  TestBed.resetTestingModule();
  const repository = new FakeAttachmentsRepository(attachments);
  const dialog = new FakeFileDialog();
  TestBed.configureTestingModule({
    providers: [provideAppTesting({ attachmentsRepository: repository, fileDialog: dialog })],
  });

  return {
    store: TestBed.inject(AttachmentsStore),
    repository,
    dialog,
    notifier: TestBed.inject(ErrorNotifier),
    status: TestBed.inject(StatusNotifier),
  };
}

describe('AttachmentsStore', () => {
  let harness: Harness;

  beforeEach(() => {
    harness = createStore([attachment(), attachment({ id: 'attachment-2', noteId: 'note-2' })]);
  });

  it('loads only the attachments of the open note', async () => {
    await harness.store.openFor('note-1');

    expect(harness.store.attachments().map((item) => item.id)).toEqual(['attachment-1']);
    expect(harness.store.count()).toBe(1);
  });

  it('empties itself when no note is open', async () => {
    await harness.store.openFor('note-1');

    await harness.store.openFor(null);

    expect(harness.store.attachments()).toEqual([]);
  });

  it('does not reload when the same note is reopened', async () => {
    await harness.store.openFor('note-1');
    await harness.store.togglePreview('attachment-1');

    await harness.store.openFor('note-1');

    expect(harness.store.previewId()).toBe('attachment-1');
  });

  it('drops the preview when the note changes', async () => {
    await harness.store.openFor('note-1');
    await harness.store.togglePreview('attachment-1');

    await harness.store.openFor('note-2');

    expect(harness.store.previewId()).toBeNull();
    expect(harness.store.previewData()).toBeNull();
  });

  it('reads the bytes only when a preview is asked for', async () => {
    await harness.store.openFor('note-1');

    expect(harness.repository.reads).toEqual([]);

    await harness.store.togglePreview('attachment-1');
    expect(harness.repository.reads).toEqual(['attachment-1']);
    expect(harness.store.previewData()).toContain('data:image/png;base64,');
  });

  it('names the attachment being previewed, for the enlarged view', async () => {
    await harness.store.openFor('note-1');
    expect(harness.store.previewed()).toBeNull();

    await harness.store.togglePreview('attachment-1');

    expect(harness.store.previewed()?.fileName).toBe('capture.png');
  });

  it('closes an open preview instead of re-reading it', async () => {
    await harness.store.openFor('note-1');
    await harness.store.togglePreview('attachment-1');

    await harness.store.togglePreview('attachment-1');

    expect(harness.store.previewId()).toBeNull();
    expect(harness.repository.reads).toHaveLength(1);
  });

  it('adds nothing when the file picker is cancelled', async () => {
    await harness.store.openFor('note-1');
    harness.dialog.openPath = null;

    expect(await harness.store.attach()).toBe(false);
    expect(harness.store.count()).toBe(1);
  });

  it('attaches the chosen file to the open note', async () => {
    await harness.store.openFor('note-1');
    harness.dialog.openPath = 'C:/shots/error.png';

    expect(await harness.store.attach()).toBe(true);
    expect(harness.store.attachments().map((item) => item.fileName)).toEqual(['capture.png', 'error.png']);
  });

  it('refuses to attach when no note is open', async () => {
    harness.dialog.openPath = 'C:/shots/error.png';

    expect(await harness.store.attach()).toBe(false);
  });

  it('removes an attachment and closes its preview', async () => {
    await harness.store.openFor('note-1');
    await harness.store.togglePreview('attachment-1');

    expect(await harness.store.remove('attachment-1')).toBe(true);
    expect(harness.store.count()).toBe(0);
    expect(harness.store.previewId()).toBeNull();
  });

  it('surfaces a failed read and shows no half-open preview', async () => {
    await harness.store.openFor('note-1');
    harness.repository.failNext = new Error('boom');

    await harness.store.togglePreview('attachment-1');

    expect(harness.store.previewId()).toBeNull();
    expect(harness.notifier.notice()?.ref.key).toBe('errors.attachmentReadFailed');
  });

  it('surfaces a failed attach', async () => {
    await harness.store.openFor('note-1');
    harness.dialog.openPath = 'C:/shots/error.png';
    harness.repository.failNext = new Error('boom');

    expect(await harness.store.attach()).toBe(false);
    expect(harness.notifier.notice()?.ref.key).toBe('errors.attachFailed');
    expect(harness.store.isBusy()).toBe(false);
  });
  describe('getting a file back out', () => {
    it('opens it with the system application', async () => {
      await harness.store.openFor('note-1');

      await harness.store.open('attachment-1');

      expect(harness.repository.opened).toEqual(['attachment-1']);
    });

    it('surfaces a refusal to open rather than doing nothing visible', async () => {
      await harness.store.openFor('note-1');
      harness.repository.failNext = new Error('no handler');

      await harness.store.open('attachment-1');

      expect(harness.notifier.notice()?.ref.key).toBe('errors.attachmentOpenFailed');
    });

    it('proposes the original name when saving elsewhere', async () => {
      await harness.store.openFor('note-1');
      harness.dialog.savePath = 'C:/out/capture.png';

      const saved = await harness.store.saveAs('attachment-1');

      expect(harness.dialog.saveCalls[0].defaultPath).toBe('capture.png');
      expect(harness.repository.savedAs).toEqual({
        id: 'attachment-1',
        path: 'C:/out/capture.png',
      });
      expect(saved).toBe('C:/out/capture.png');
    });

    it('writes nothing when the save dialog is cancelled', async () => {
      await harness.store.openFor('note-1');
      harness.dialog.savePath = null;

      expect(await harness.store.saveAs('attachment-1')).toBeNull();
      expect(harness.repository.savedAs).toBeNull();
    });

    it('does not open a save dialog for an attachment it does not hold', async () => {
      await harness.store.openFor('note-1');

      expect(await harness.store.saveAs('ghost')).toBeNull();
      expect(harness.dialog.saveCalls).toHaveLength(0);
    });
  });

  describe('the other two ways to attach', () => {
    it('takes a dropped path without reopening the picker', async () => {
      await harness.store.openFor('note-1');

      expect(await harness.store.attachPath('C:/shots/error.png')).toBe(true);
      expect(harness.dialog.openCalls).toHaveLength(0);
      expect(harness.store.attachments().map((item) => item.fileName)).toContain('error.png');
    });

    it('attaches the clipboard image under a dated name', async () => {
      await harness.store.openFor('note-1');

      expect(await harness.store.attachClipboardImage(new Date('2026-08-27T09:30:00Z'))).toBe(true);
      expect(harness.store.attachments().map((item) => item.fileName)).toContain(
        'capture-2026-08-27-09-30-00.png',
      );
    });

    it('says what was attached, and shows an image straight away', async () => {
      await harness.store.openFor('note-1');

      await harness.store.attachPath('C:/shots/error.png');

      expect(harness.status.status()).toEqual({
        key: 'attachments.added',
        params: { name: 'error.png' },
      });
      expect(harness.store.previewId()).not.toBeNull();
    });

    it('refuses every route while a note is only a draft', async () => {
      expect(await harness.store.attachPath('C:/shots/error.png')).toBe(false);
      expect(await harness.store.attachClipboardImage(new Date())).toBe(false);
    });
  });
});
