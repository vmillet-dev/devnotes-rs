import { DestroyRef, Injectable, computed, effect, inject, signal } from '@angular/core';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { FileDialogService } from '@core/services/dialogs/file-dialog.service';
import { StatusNotifier } from '@core/services/notifications/status.service';
import { ClockService } from '@core/services/time/clock.service';
import { FileDropService } from '@core/services/window/file-drop.service';
import { AttachmentsRepository } from '@core/data/attachments.repository';
import { Attachment } from '@core/model/note.model';
import { NotesStore } from './notes.store';

/**
 * Follows the open note itself rather than being told to. ⚠️ The bytes are never loaded
 * in bulk: `preview` asks for one at a time, and a `data:` URI weighs a third more
 * than the file.
 */
@Injectable({ providedIn: 'root' })
export class AttachmentsStore {
  private readonly repository = inject(AttachmentsRepository);
  private readonly dialog = inject(FileDialogService);
  private readonly status = inject(StatusNotifier);
  private readonly notifier = inject(ErrorNotifier);
  private readonly clock = inject(ClockService);
  private readonly notes = inject(NotesStore);

  private readonly _noteId = signal<string | null>(null);
  private readonly _attachments = signal<readonly Attachment[]>([]);
  private readonly _isBusy = signal(false);
  private readonly _previewId = signal<string | null>(null);
  private readonly _previewData = signal<string | null>(null);
  private readonly _zoomed = signal(false);

  readonly zoomed = this._zoomed.asReadonly();

  readonly attachments = this._attachments.asReadonly();
  readonly isBusy = this._isBusy.asReadonly();
  readonly previewId = this._previewId.asReadonly();
  readonly previewData = this._previewData.asReadonly();
  readonly count = computed(() => this._attachments().length);

  /** Resolved here: the lightbox lives in the page and cannot see the strip's state. */
  readonly previewed = computed<Attachment | null>(() => {
    const id = this._previewId();
    return this._attachments().find((attachment) => attachment.id === id) ?? null;
  });

  constructor() {
    // Attachments follow the persisted note: a draft has no row to carry them.
    effect(() => void this.openFor(this.notes.persistedNoteId()));

    // ⚠️ A drop is a window event, not a DOM one.
    const drops = inject(FileDropService);
    inject(DestroyRef).onDestroy(drops.on((paths) => void this.addDroppedFiles(paths)));
  }

  zoom(): void {
    this._zoomed.set(true);
  }

  closeZoom(): void {
    this._zoomed.set(false);
  }

  async addFromPicker(): Promise<void> {
    if (await this.targetNote()) {
      await this.attach();
    }
  }

  async addPastedImage(): Promise<void> {
    if (!(await this.targetNote())) return;

    if (!(await this.attachClipboardImage(this.clock.now()))) {
      this.notifier.notify({ ref: { key: 'attachments.pasteEmpty' } });
    }
  }

  async saveToDisk(id: string): Promise<void> {
    const path = await this.saveAs(id);
    if (path) {
      this.status.notify({ key: 'attachments.saved', params: { path } });
    }
  }

  private async addDroppedFiles(paths: readonly string[]): Promise<void> {
    if (this.notes.selectedNote() === null || paths.length === 0) return;
    if (!(await this.targetNote())) return;

    for (const path of paths) {
      await this.attachPath(path);
    }
  }

  /**
   * ⚠️ Re-pointed here rather than by the effect on `persistedNoteId`, which only runs
   * on the next detection cycle — after the write that follows, which would then attach
   * nothing and not say so. `openFor` is idempotent.
   */
  private async targetNote(): Promise<string | null> {
    const noteId = await this.notes.materialiseDraft();
    if (noteId) {
      await this.openFor(noteId);
    }

    return noteId;
  }

  /** A different note clears the preview: the previous screenshot would be worse than nothing. */
  async openFor(noteId: string | null): Promise<void> {
    if (this._noteId() === noteId) return;

    this._noteId.set(noteId);
    this.closePreview();
    this._attachments.set([]);
    if (noteId !== null) {
      await this.load(noteId);
    }
  }

  /** `false` when nothing was added — a cancelled picker included. */
  async attach(): Promise<boolean> {
    const path = await this.dialog.pickAttachment();
    if (path === null) return false;

    return this.attachPath(path);
  }

  /** The file name is announced: otherwise attaching is only visible in the strip. */
  private async write(action: (noteId: string) => Promise<Attachment>): Promise<boolean> {
    const noteId = this._noteId();
    if (noteId === null || this._isBusy()) return false;

    const added = await this.notifier.attemptWhile(this._isBusy, 'errors.attachFailed', () => action(noteId));
    if (added === null) return false;

    await this.load(noteId);
    this.status.notify({ key: 'attachments.added', params: { name: added.fileName } });
    if (added.mimeType.startsWith('image/')) {
      await this.togglePreview(added.id);
    }

    return true;
  }

  /** An already named file — the one just dropped: the picker is not reopened. */
  async attachPath(path: string): Promise<boolean> {
    return this.write((noteId) => this.repository.attach(noteId, path));
  }

  /** `Ctrl+V` in the editor comes through here when the clipboard holds no text. */
  async attachClipboardImage(now: Date): Promise<boolean> {
    return this.write((noteId) => this.repository.attachClipboardImage(noteId, screenshotName(now)));
  }

  async open(id: string): Promise<void> {
    await this.notifier.attempt('errors.attachmentOpenFailed', () => this.repository.open(id));
  }

  /** `null` when nothing was saved — a cancelled picker included. */
  async saveAs(id: string): Promise<string | null> {
    const attachment = this._attachments().find((candidate) => candidate.id === id);
    if (!attachment) return null;

    const path = await this.dialog.chooseDestination(attachment.fileName);
    if (path === null) return null;

    const saved = await this.notifier.attempt('errors.attachmentSaveFailed', () =>
      this.repository.saveAs(id, path),
    );

    return saved === null ? null : path;
  }

  async remove(id: string): Promise<boolean> {
    const noteId = this._noteId();
    if (noteId === null) return false;

    const removed = await this.notifier.attempt('errors.attachmentDeleteFailed', () =>
      this.repository.delete(id),
    );
    if (removed === null) return false;

    if (this._previewId() === id) {
      this.closePreview();
    }
    await this.load(noteId);
    return true;
  }

  /** A toggle: asking again for the open preview closes it, with no round trip. */
  async togglePreview(id: string): Promise<void> {
    this._zoomed.set(false);
    if (this._previewId() === id) {
      this.closePreview();
      return;
    }

    this._previewId.set(id);
    this._previewData.set(null);
    const data = await this.notifier.attempt('errors.attachmentReadFailed', () => this.repository.read(id));
    if (data === null) {
      this.closePreview();
      return;
    }
    // The note may have changed during the read: show only what is still asked for.
    if (this._previewId() === id) {
      this._previewData.set(data);
    }
  }

  closePreview(): void {
    this._previewId.set(null);
    this._previewData.set(null);
    this._zoomed.set(false);
  }

  private async load(noteId: string): Promise<void> {
    const loaded = await this.notifier.attempt('errors.attachFailed', () => this.repository.loadFor(noteId));
    if (loaded) this._attachments.set(loaded);
  }
}

/** Dated to the second; the extension is added on the Rust side. */
function screenshotName(now: Date): string {
  const stamp = now.toISOString().slice(0, 19).replace(/[:T]/g, '-');
  return `capture-${stamp}`;
}
